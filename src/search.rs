use crate::{
    engine::Engine,
    evaluation::evaluate,
    items::{Color, Move},
    tt::{TTEntry, TTFlag},
    uci::{GoControl, MAX_DEPTH},
    uci_print,
};

use std::time::{Duration, Instant};

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
            score: 0,
            nodes: 0,
            abort: false,
        };

        let mut last_complete_info = info.clone();
        for d in 1..=limits.depth.unwrap_or(MAX_DEPTH) {
            let mut best_move = Move::NULL;
            let mut best_score = -INF;

            let mut move_list = self.board.gen_moves();

            let mut tt_move = Move::NULL;
            if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
                tt_move = entry.best_move;
            }

            for mv in move_list.with_ordering(tt_move) {
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
            return 0;
        }

        info.nodes += 1;

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
            return self.quiescence(alpha, beta, info, limits);
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

        for (mv_idx, mv) in move_list.with_ordering(tt_move).enumerate() {
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
        info: &mut SearchInfo,
        limits: &SearchLimits,
    ) -> i32 {
        info.check_limits(limits);

        if info.abort {
            return 0;
        }

        info.nodes += 1;

        // Stand pat
        let stand_pat = evaluate(&self.board);

        if stand_pat >= beta {
            return beta;
        }

        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let mut move_list = self.board.gen_moves();

        let mut tt_move = Move::NULL;
        if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
            tt_move = entry.best_move;
        }

        for mv in move_list.with_ordering(tt_move) {
            // Only tactical moves
            if !mv.flag().is_capture() && !mv.flag().is_promo() {
                continue;
            }

            let undo = self.board.make_move(&mv);
            let score = -self.quiescence(-beta, -alpha, info, limits);
            self.board.unmake_move(&mv, &undo);

            if info.abort {
                return 0;
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
    pub score: i32,
    pub best_move: Move,
    pub nodes: u64,
    pub abort: bool,
}

impl SearchInfo {
    pub fn print(&self) {
        uci_print!(
            "info depth {} score cp {} nodes {} nps {} time {} pv {}",
            self.depth,
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
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            soft_time: None,
            hard_time: None,
            depth: None,
            infinite: false,
            start_time: Instant::now(),
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

        if time_left > 0 {
            let moves_to_go = ctrl.movestogo.unwrap_or(30);

            let allocated = time_left / moves_to_go + increment / 2;

            limits.soft_time = Some(Duration::from_millis(allocated * 1 / 2));

            limits.hard_time = Some(Duration::from_millis(allocated * 9 / 10));
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
