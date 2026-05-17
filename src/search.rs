use crate::{
    board::{Board, mask},
    r#const::{BLACK_PAWN_ATTACKS, KING_ATTACKS, KNIGHT_ATTACKS, WHITE_PAWN_ATTACKS},
    engine::Engine,
    evaluation::evaluate,
    items::{Color, Move, MoveFlag, Piece, PieceInfo},
    square::Square,
    tt::{TTEntry, TTFlag},
    uci::{GoControl, MAX_DEPTH},
    uci_print,
};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const INF: i32 = 10000000;

impl Engine {
    pub fn search_ids<F>(&mut self, limits: &SearchLimits, mut on_iteration: F) -> SearchInfo
    where
        F: FnMut(&SearchInfo),
    {
        let mut info = SearchInfo {
            start_time: Instant::now(),
            best_move: Move::NULL,
            depth: 0,
            seldepth: 0,
            score: 0,
            nodes: 0,
            abort: false,
            is_mandatory: true,
        };

        let mut last_complete_info = info.clone();
        for d in 1..=limits.depth.unwrap_or(MAX_DEPTH) {
            let mut best_move = Move::NULL;
            let mut best_score = -INF;

            // making the search of depth 1 completely mandatory
            // as it guarentees us to return a valid move
            info.is_mandatory = 1 == d;

            let mut move_list = self.board.gen_moves();

            let mut tt_move = Move::NULL;
            if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
                tt_move = entry.best_move;
            }

            for mv in move_list.with_ordering(tt_move, &self.board) {
                let undo = self.board.make_move(&mv);
                let score = -self.negamax(d - 1, -INF, INF, 1, &limits, &mut info);
                self.board.unmake_move(&mv, &undo);

                if info.abort {
                    break;
                }

                if score > best_score {
                    best_score = score;
                    best_move = mv;
                }
            }

            // if aborted, dont update the result
            if info.abort {
                break;
            }

            if best_move == Move::NULL && move_list.len() > 0 {
                best_move = move_list.get(0);
            }

            info.depth = d;
            info.score = best_score;
            info.best_move = best_move;

            last_complete_info = info.clone();

            on_iteration(&info);

            // if it reached the solf limit, searching further is probably useless
            if let Some(time) = limits.soft_time {
                if time <= info.start_time.elapsed() {
                    break;
                }
            }
        }

        last_complete_info
    }

    fn negamax(
        &mut self,
        depth: u16,
        mut alpha: i32,
        mut beta: i32,
        ply: i32,
        limits: &SearchLimits,
        info: &mut SearchInfo,
    ) -> i32 {
        info.check_limits(limits);

        if info.abort {
            return alpha;
        }

        info.nodes += 1;
        info.seldepth = info.seldepth.max(ply as u16);

        // Checking draws
        if self.board.is_threefold() || self.board.is_50_rule() {
            return 0;
        }

        // Probing the TT
        let key = self.board.get_zob_key();
        let mut tt_move = Move::NULL;

        if let Some(entry) = self.tt.probe(key) {
            tt_move = entry.best_move;
            let mut score = entry.score;

            // De-adjust mate score
            if entry.score > INF - 1000 {
                score -= ply;
            }

            if entry.score < -INF + 1000 {
                score += ply;
            }

            if entry.depth >= depth {
                match entry.flag {
                    TTFlag::Exact => {
                        return score;
                    }
                    TTFlag::LowerBound => alpha = alpha.max(score),
                    TTFlag::UpperBound => beta = beta.min(score),
                }

                if alpha >= beta {
                    return score;
                }
            }
        }

        // base case handling
        if depth == 0 {
            return self.quiescence(alpha, beta, ply, info, limits);
        }

        let mut move_list = self.board.gen_moves();
        let original_alpha = alpha;

        // checking mates
        if move_list.len() == 0 {
            return if self.board.in_check() { -INF + ply } else { 0 };
        }

        // NULL move pruning
        if let Some(cutoff_score) = self.nmp_search(depth, beta, ply, limits, info) {
            return cutoff_score;
        }

        let mut max_eval = -INF;
        let mut best_move_this_node = Move::NULL;

        for (mv_idx, mv) in move_list.with_ordering(tt_move, &self.board).enumerate() {
            let undo = self.board.make_move(&mv);
            // Late Move Reduction (LMR)
            let eval = self.lmr_search(&mv, mv_idx, depth, alpha, beta, ply, limits, info);

            self.board.unmake_move(&mv, &undo);

            if eval > max_eval {
                max_eval = eval;
                best_move_this_node = mv;
            }

            if eval > alpha {
                alpha = eval;
            }

            // pruning
            if alpha >= beta {
                break;
            }
        }

        let flag = if max_eval <= original_alpha {
            TTFlag::UpperBound
        } else if max_eval >= beta {
            TTFlag::LowerBound
        } else {
            TTFlag::Exact
        };

        // Adjusting for mate score
        let mut score_to_store = max_eval;
        if score_to_store > INF - 1000 {
            score_to_store += ply;
        }
        if score_to_store < -INF + 1000 {
            score_to_store -= ply;
        }

        if !info.abort {
            self.tt.store(TTEntry {
                key,
                depth: depth,
                score: score_to_store,
                flag,
                best_move: best_move_this_node,
            });
        }

        max_eval
    }

    fn nmp_search(
        &mut self,
        depth: u16,
        beta: i32,
        ply: i32,
        limits: &SearchLimits,
        info: &mut SearchInfo,
    ) -> Option<i32> {
        // Conditions for NMP
        if depth > 4
            && !self.board.in_check()
            && !self.board.is_endgame()
            && evaluate(&self.board) >= beta
        {
            let r = 2 + depth / 6;
            let old_epsq = self.board.make_null_move();

            // Zero-window search
            let score = -self.negamax(depth - 1 - r, -beta, -beta + 1, ply + 1, limits, info);

            self.board.unmake_null_move(old_epsq);

            if score >= beta {
                // Not returning mate scores from NMP as it can lead to false mates
                return Some(if score >= INF - 1000 { beta } else { score });
            }
        }

        None
    }

    fn lmr_search(
        &mut self,
        mv: &Move,
        mv_idx: usize,
        depth: u16,
        alpha: i32,
        beta: i32,
        ply: i32,
        limits: &SearchLimits,
        info: &mut SearchInfo,
    ) -> i32 {
        let in_check = self.board.in_check();

        // Check if move is eligible for LMR
        if mv_idx > 3 && depth > 4 && !in_check && !mv.flag().is_capture() && !mv.flag().is_promo()
        {
            let mut reduction = 1 + (mv_idx as u16 / 4) + (depth / 6);
            if depth <= 5 {
                reduction = 1;
            }
            let reduction = reduction.min(depth - 1);

            // Search at reduced depth with a null window
            let mut eval = -self.negamax(
                depth - 1 - reduction,
                -alpha - 1,
                -alpha,
                ply + 1,
                limits,
                info,
            );

            // If reduced search fails high, we must re-search at full depth
            if eval > alpha {
                eval = -self.negamax(depth - 1, -beta, -alpha, ply + 1, limits, info);
            }
            eval
        } else {
            // Normal PVS/Negamax search
            -self.negamax(depth - 1, -beta, -alpha, ply + 1, limits, info)
        }
    }

    fn quiescence(
        &mut self,
        mut alpha: i32,
        beta: i32,
        ply: i32,
        info: &mut SearchInfo,
        limits: &SearchLimits,
    ) -> i32 {
        info.check_limits(limits);

        if info.abort {
            return alpha;
        }

        info.nodes += 1;
        info.seldepth = info.seldepth.max(ply as u16);

        let in_check = self.board.in_check();

        // Stand pat
        let stand_pat = if !in_check {
            evaluate(&self.board)
        } else {
            -INF
        };

        // beta cutoff
        if stand_pat >= beta {
            return beta;
        }

        // delta pruning
        if !in_check {
            const BIG_DELTA: i32 = 1100;
            if stand_pat < alpha - BIG_DELTA {
                return alpha;
            }
        }

        if stand_pat > alpha {
            alpha = stand_pat;
        }

        //////// NOTE: MUST BE REMOVED LATER ///////////
        // hardcoded depth cutting
        if ply - info.depth as i32 > 8 {
            return alpha;
        }
        ////////////////////////////////////////////////

        let mut move_list = if in_check {
            self.board.gen_moves()
        } else {
            self.board.gen_cap_moves()
        };

        let mut tt_move = Move::NULL;
        if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
            tt_move = entry.best_move;
        }

        for mv in move_list.with_ordering(tt_move, &self.board) {
            // soft delta pruning
            if !in_check {
                if self.board.see(&mv) < -75 && !mv.flag().is_promo() {
                    continue;
                }
            }

            let undo = self.board.make_move(&mv);
            let score = -self.quiescence(-beta, -alpha, ply + 1, info, limits);
            self.board.unmake_move(&mv, &undo);

            if info.abort {
                return alpha;
            }

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }
}

#[derive(Clone, Copy)]
pub struct SearchInfo {
    pub start_time: Instant,
    pub depth: u16,
    pub seldepth: u16,
    pub score: i32,
    pub best_move: Move,
    pub nodes: u64,
    pub abort: bool,
    pub is_mandatory: bool,
}

impl SearchInfo {
    pub fn print(&self) {
        uci_print!(
            "info depth {} seldepth {} score cp {} nodes {} nps {} time {} pv {}",
            self.depth,
            self.seldepth,
            self.score,
            self.nodes,
            self.nps(),
            self.start_time.elapsed().as_millis(),
            self.best_move.to_coord(),
        );
    }

    fn nps(&self) -> u64 {
        let ms = self.start_time.elapsed().as_millis().max(1);
        self.nodes * 1000 / ms as u64
    }

    fn check_limits(&mut self, limits: &SearchLimits) {
        // not checking time limits if it is a mandatory search
        if self.is_mandatory {
            return;
        }

        // cheking if stop command is made
        if limits.stop_signal.load(Ordering::Relaxed) {
            self.abort = true;
            return;
        }

        // checking once in a while if the time limit is reached
        if self.nodes & 2047 == 0 {
            if let Some(limit) = limits.hard_time {
                if self.start_time.elapsed() >= limit {
                    self.abort = true;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SearchLimits {
    pub depth: Option<u16>,
    pub hard_time: Option<Duration>,
    pub soft_time: Option<Duration>,
    pub infinite: bool,
    pub start_time: Instant,

    pub stop_signal: Arc<AtomicBool>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            soft_time: None,
            hard_time: None,
            depth: None,
            infinite: false,
            start_time: Instant::now(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl SearchLimits {
    pub fn from_go(ctrl: &GoControl, stm: Color) -> Self {
        let mut limits = SearchLimits::default();
        limits.depth = ctrl.depth;
        limits.infinite = ctrl.infinite;

        if ctrl.infinite {
            return limits;
        }

        // Fixed movetime
        if let Some(ms) = ctrl.movetime {
            let duration = Duration::from_millis(ms);

            limits.soft_time = Some(duration);
            limits.hard_time = Some(duration);

            return limits;
        }

        // Normal clock management
        let (time_left, increment) = match stm {
            Color::White => (ctrl.wtime.unwrap_or(0), ctrl.winc.unwrap_or(0)),
            Color::Black => (ctrl.btime.unwrap_or(0), ctrl.binc.unwrap_or(0)),
        };

        // Subtract 50ms to account for GUI/Network communication time.
        let safe_time_left = time_left.saturating_sub(50);

        if safe_time_left > 0 {
            let moves_to_go = ctrl.movestogo.unwrap_or(30);

            // Base allocation: spread remaining safe time over expected remaining moves
            let base_time = safe_time_left / moves_to_go;

            // Use 3/4 of the increment (standard aggressive time management)
            let inc_time = increment * 3 / 4;

            let allocated = base_time + inc_time;

            // Soft Limit: Stop starting new depths early (60% of allocated)
            limits.soft_time = Some(Duration::from_millis(allocated * 6 / 10));

            // Hard Limit: min 20 max 80 % of safe_time
            let hard_limit = (allocated * 2).max(20).min(safe_time_left * 8 / 10);

            limits.hard_time = Some(Duration::from_millis(hard_limit));
        } else if time_left > 0 {
            // EMERGENCY MODE: We have less than 50ms on the actual clock
            // Give it 1ms soft time, and whatever is physically left on the clock (minus a tiny 5ms buffer).
            limits.soft_time = Some(Duration::from_millis(1));
            limits.hard_time = Some(Duration::from_millis(time_left.saturating_sub(5).max(1)));
        }

        limits
    }

    pub fn with_depth(depth: u16) -> Self {
        let mut limits = Self::default();
        limits.depth = Some(depth);
        limits
    }

    pub fn with_movetime(movetime: u64) -> Self {
        let mut limits = Self::default();
        limits.soft_time = Some(Duration::from_millis(movetime));
        limits.hard_time = Some(Duration::from_millis(movetime));
        limits
    }
}

impl Board {
    fn attacks_to(&self, to_pos: usize, all_occ: u64) -> u64 {
        let mut attackers = 0u64;

        // Sliding pieces
        let directions = [
            ([(1, 1), (1, -1), (-1, 1), (-1, -1)], true), // diagonals
            ([(1, 0), (-1, 0), (0, 1), (0, -1)], false),  // straight
        ];
        let from = Square::new(to_pos);

        for (dir, is_diag) in directions {
            for (dr, df) in dir {
                let mut sq = from;

                while let Some(next) = sq.offset(dr, df) {
                    let to_bb = mask(next.index());

                    // Some piece is blocking our way
                    if to_bb & all_occ != 0 {
                        let piece = self.piece_on(next.index());
                        let piece_type = Piece::get_type(piece);
                        if piece_type == Piece::QUEEN
                            || (piece_type == Piece::BISHOP && is_diag)
                            || (piece_type == Piece::ROOK && !is_diag)
                        {
                            attackers |= to_bb;
                        }

                        // accumulate attackers if the blocking piece is an enemy rook,
                        // bishop or a queen, else break the loop as we have
                        // been blocked by our own piece, or an non sliding
                        // enemy piece

                        break;
                    }
                    sq = next;
                }
            }
        }

        //knight is attacking
        attackers |= KNIGHT_ATTACKS[to_pos]
            & (self.bb(Piece::WHITE | Piece::KNIGHT) | self.bb(Piece::BLACK | Piece::KNIGHT))
            & all_occ;

        // King attacks
        attackers |= KING_ATTACKS[to_pos]
            & (self.bb(Piece::WHITE | Piece::KING) | self.bb(Piece::BLACK | Piece::KING))
            & all_occ;

        // if a opp pawn is in cur color pawn's attacking sq, then
        // the opponent pawn is attacking the current sq
        attackers |= WHITE_PAWN_ATTACKS[to_pos] & self.bb(Piece::BLACK | Piece::PAWN) & all_occ;
        attackers |= BLACK_PAWN_ATTACKS[to_pos] & self.bb(Piece::WHITE | Piece::PAWN) & all_occ;

        attackers
    }

    // Code written by refering the algorithm provided in chess programming wiki
    // link -> https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    pub fn see(&self, mov: &Move) -> i32 {
        if mov.flag() == MoveFlag::EN_PASSANT {
            return 100;
        }

        let mut gain = [0; 32];
        let mut d = 0;

        let from = mov.from();
        let to = mov.to();

        let target = self.piece_on(to);
        let mut cur_victim = self.piece_on(from); // victim coz it moved to the to_sq

        // Initial gain = captured piece
        gain[0] = self.get_see_value(target);

        // Simulated occupancy AFTER first capture
        let mut occ = self.all_occ();
        occ ^= mask(from); // piece moved from from_sq

        let mut side = self.side_to_move().opponent();

        loop {
            // Find all attackers in current position
            let attackers = self.attacks_to(to, occ);
            let (from_set, piece) = self.get_least_valuable_piece(attackers, occ, side);

            if from_set == 0 {
                break;
            }

            d += 1;

            gain[d] = self.get_see_value(cur_victim) - gain[d - 1];

            // SEE pruning
            if (-gain[d - 1]).max(gain[d]) < 0 {
                break;
            }

            cur_victim = piece; // next victim is the cur attacker

            // Remove this attacker from occupancy
            occ ^= from_set;
            side = side.opponent();
        }

        // Backward minimax pass
        while d > 0 {
            d -= 1;
            gain[d] = -((-gain[d]).max(gain[d + 1]));
        }

        gain[0]
    }

    fn get_least_valuable_piece(&self, attackers: u64, occ: u64, side: Color) -> (u64, PieceInfo) {
        let color_mask = if side == Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let my_attackers = attackers & occ & self.occ(&side);

        for p_type in [
            Piece::PAWN,
            Piece::KNIGHT,
            Piece::BISHOP,
            Piece::ROOK,
            Piece::QUEEN,
            Piece::KING,
        ] {
            let subset = my_attackers & self.bb(p_type | color_mask);

            if subset != 0 {
                let lsb = subset & subset.wrapping_neg();

                return (lsb, p_type | color_mask);
            }
        }

        (0, Piece::NONE)
    }

    fn get_see_value(&self, piece: PieceInfo) -> i32 {
        if piece == Piece::NONE {
            return 0;
        }

        let idx = Piece::to_idx(piece) % 6;
        // Piece indices: 0:P, 1:N, 2:B, 3:R, 4:Q, 5:K
        match idx {
            0 => 100,
            1 => 320,
            2 => 330,
            3 => 500,
            4 => 900,
            _ => 10000,
        }
    }
}
