use crate::{
    engine::Engine,
    evaluation::evaluate,
    items::Move,
    tt::{TTEntry, TTFlag},
    uci_print,
};

use std::time::{Duration, Instant};

const INF: i32 = 10000000;

impl Engine {
    pub fn search_ids<F>(
        &mut self,
        depth: u16,
        time_limit: Option<Duration>,
        mut on_iteration: F,
    ) -> SearchInfo
    where
        F: FnMut(&SearchInfo),
    {
        let mut info = SearchInfo {
            start_time: Instant::now(),
            best_move: Move::NULL,
            depth: 0,
            score: 0,
            time_limit,
            nodes: 0,
            abort: false,
        };

        let mut last_complete_info = info.clone();
        for d in 1..=depth {
            let mut best_move = Move::NULL;
            let mut best_score = -INF;

            let mut move_list = self.board.gen_moves();

            let mut tt_move = Move::NULL;
            if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
                tt_move = entry.best_move;
            }

            for mv in move_list.with_ordering(tt_move) {
                let undo = self.board.make_move(&mv);
                let score = -self.negamax(d - 1, -INF, INF, 1, &mut info);
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
        }

        last_complete_info
    }

    fn negamax(
        &mut self,
        depth: u16,
        mut alpha: i32,
        mut beta: i32,
        ply: i32,
        info: &mut SearchInfo,
    ) -> i32 {
        info.check_limits();

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

            if entry.depth >= depth as i32 {
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
            return evaluate(&self.board);
        }

        let mut move_list = self.board.gen_moves();
        let original_alpha = alpha;

        // checking mates
        if move_list.len() == 0 {
            return if self.board.in_check() { -INF + ply } else { 0 };
        }

        // NULL move pruning
        if let Some(cutoff_score) = self.nmp_search(depth, beta, ply, info) {
            return cutoff_score;
        }

        let mut max_eval = -INF;
        let mut best_move_this_node = Move::NULL;

        for (mv_idx, mv) in move_list.with_ordering(tt_move).enumerate() {
            let undo = self.board.make_move(&mv);
            // Late Move Reduction (LMR)
            let eval = self.lmr_search(&mv, mv_idx, depth, alpha, beta, ply, info);

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
                depth: depth as i32,
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
            let score = -self.negamax(depth - 1 - r, -beta, -beta + 1, ply + 1, info);

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
            let mut eval = -self.negamax(depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, info);

            // If reduced search fails high, we must re-search at full depth
            if eval > alpha {
                eval = -self.negamax(depth - 1, -beta, -alpha, ply + 1, info);
            }
            eval
        } else {
            // Normal PVS/Negamax search
            -self.negamax(depth - 1, -beta, -alpha, ply + 1, info)
        }
    }
}

#[derive(Clone, Copy)]
pub struct SearchInfo {
    pub start_time: Instant,
    pub depth: u16,
    pub score: i32,
    pub best_move: Move,
    pub nodes: u64,
    pub time_limit: Option<Duration>,
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

    fn check_limits(&mut self) {
        if self.nodes & 2047 == 0 {
            if let Some(limit) = self.time_limit {
                if self.start_time.elapsed() >= limit {
                    self.abort = true;
                }
            }
        }
    }
}
