use crate::{
    board::Board,
    evaluation::evaluate,
    items::Move,
    transposition_table::{TTEntry, TTFlag, TranspositionTable},
};

use std::cell::Cell;

const INF: i32 = 30000;

thread_local! {
    static NODES: Cell<u64> = Cell::new(0);
}

#[inline(always)]
fn inc_nodes() {
    NODES.with(|n| n.set(n.get() + 1));
}

pub fn find_best_move(board: &mut Board, depth: u16) -> (Option<Move>, (u64, i32)) {
    NODES.with(|n| n.set(0)); // reset

    let mut tt = TranspositionTable::new(16);
    let mut best_move = None;
    let mut best_score = -INF;

    let mut move_list = board.gen_moves();

    for mv in move_list.with_ordering() {
        let undo = board.make_move(&mv);
        let cur_score = -negamax(board, depth - 1, -INF, INF, &mut tt);
        board.unmake_move(&mv, &undo);

        if cur_score > best_score {
            best_score = cur_score;
            best_move = Some(mv);
        }
    }

    let nodes = NODES.with(|n| n.get());
    (best_move, (nodes, best_score))
}

pub fn negamax(
    board: &mut Board,
    depth: u16,
    mut alpha: i32,
    mut beta: i32,
    tt: &mut TranspositionTable,
) -> i32 {
    inc_nodes();

    let key = board.get_zob_key();

    if let Some(entry) = tt.probe(key) {
        if entry.depth >= depth as i32 {
            match entry.flag {
                TTFlag::Exact => return entry.score,
                TTFlag::LowerBound => alpha = alpha.max(entry.score),
                TTFlag::UpperBound => beta = beta.min(entry.score),
            }

            if alpha >= beta {
                return entry.score;
            }
        }
    }

    if board.is_threefold() || board.is_50_rule() {
        return 0;
    }

    if depth == 0 {
        return evaluate(board);
    }

    let mut move_list = board.gen_moves();
    let original_alpha = alpha;

    if move_list.len() == 0 {
        if board.in_check() {
            return -INF + depth as i32;
        } else {
            return 0;
        }
    }
    let mut max_eval = -INF;

    for mv in move_list.with_ordering() {
        let undo = board.make_move(&mv);
        let eval = -negamax(board, depth - 1, -beta, -alpha, tt);
        board.unmake_move(&mv, &undo);

        if eval > max_eval {
            max_eval = eval;
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

    tt.store(TTEntry {
        key,
        depth: depth as i32,
        score: max_eval,
        flag,
        best_move: Move::NULL,
    });

    max_eval
}
