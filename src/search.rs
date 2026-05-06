use crate::{
    board::Board,
    evaluation::evaluate,
    items::Move,
    transposition_table::{TTEntry, TTFlag, TranspositionTable},
    uci_print,
};

use std::time::Instant;

const INF: i32 = 10000000;

pub fn search_ids(board: &mut Board, depth: u16, tt: &mut TranspositionTable) -> SearchInfo {
    let mut info = SearchInfo {
        start_time: Instant::now(),
        best_move: Move::NULL,
        depth: 0,
        score: 0,
        nodes: 0,
    };

    for d in 1..=depth {
        let mut best_move = Move::NULL;
        let mut best_score = -INF;

        let mut move_list = board.gen_moves();

        let mut tt_move = Move::NULL;
        if let Some(entry) = tt.probe(board.get_zob_key()) {
            tt_move = entry.best_move;
        }

        for mv in move_list.with_ordering(tt_move) {
            let undo = board.make_move(&mv);
            let score = -negamax(board, d - 1, -INF, INF, 1, tt, &mut info);
            board.unmake_move(&mv, &undo);

            if score > best_score {
                best_score = score;
                best_move = mv;
            }
        }

        if best_move == Move::NULL && move_list.len() > 0 {
            best_move = move_list.get(0);
        }

        info.depth = d;
        info.score = best_score;
        info.best_move = best_move;

        info.print();
    }

    info
}

fn negamax(
    board: &mut Board,
    depth: u16,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
    info: &mut SearchInfo,
) -> i32 {
    info.nodes += 1;

    // Checking draws
    if board.is_threefold() || board.is_50_rule() {
        return 0;
    }

    // Probing the TT
    let key = board.get_zob_key();
    let mut tt_move = Move::NULL;

    if let Some(entry) = tt.probe(key) {
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
        return evaluate(board);
    }

    let mut move_list = board.gen_moves();
    let original_alpha = alpha;

    // checking mates
    if move_list.len() == 0 {
        return if board.in_check() { -INF + ply } else { 0 };
    }

    // NULL move pruning
    let eval = evaluate(board);
    if depth >= 4 && !board.in_check() && !board.is_endgame() && eval >= beta {
        let r = 2 + depth / 6;
        let old_epsq = board.make_null_move();
        let score = -negamax(board, depth - 1 - r, -beta, -beta + 1, ply + 1, tt, info);
        board.unmake_null_move(old_epsq);

        if score >= beta {
            // Not returning mate scores from NMP as it can lead to false mates
            return if score >= INF - 1000 { beta } else { score };
        }
    }

    let mut max_eval = -INF;
    let mut best_move_this_node = Move::NULL;

    for (mv_idx, mv) in move_list.with_ordering(tt_move).enumerate() {
        let mut eval;
        let in_check = board.in_check();
        let undo = board.make_move(&mv);

        // Last Move Reduction (LMR)
        // Only reduce if: not in check, not a capture, not a promotion, and i > 3
        if mv_idx > 3 && depth >= 4 && !in_check && !mv.flag().is_capture() && !mv.flag().is_promo()
        {
            let mut reduction: u16 = 1 + (mv_idx as u16 / 4) + (depth / 6);
            if depth <= 5 {
                reduction = 1;
            }

            reduction = reduction.min(depth - 1);

            // searching at reduced depth
            eval = -negamax(
                board,
                depth - 1 - reduction,
                -alpha - 1,
                -alpha,
                ply + 1,
                tt,
                info,
            );

            // researching if the reduced depth search is actually useful
            if eval > alpha {
                eval = -negamax(board, depth - 1, -beta, -alpha, ply + 1, tt, info);
            }
        } else {
            // Normal search for first few moves and tactical moves
            let score = negamax(board, depth - 1, -beta, -alpha, ply + 1, tt, info);
            eval = -score;
        }

        board.unmake_move(&mv, &undo);

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

    tt.store(TTEntry {
        key,
        depth: depth as i32,
        score: score_to_store,
        flag,
        best_move: best_move_this_node,
    });

    max_eval
}
pub struct SearchInfo {
    pub start_time: Instant,
    pub depth: u16,
    pub score: i32,
    pub best_move: Move,
    pub nodes: u64,
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
}
