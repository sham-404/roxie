use crate::{
    board::Board,
    evaluation::evaluate,
    items::{Color, Move},
};

use std::cell::Cell;

thread_local! {
    static NODES: Cell<u64> = Cell::new(0);
}

#[inline(always)]
fn inc_nodes() {
    NODES.with(|n| n.set(n.get() + 1));
}

pub fn find_best_move(board: &mut Board, depth: u16) -> (Option<Move>, u64) {
    NODES.with(|n| n.set(0)); // reset

    let mut best_move = None;
    let mut best_score = i32::MIN;

    let move_list = board.gen_moves();

    for mv in move_list.as_slice() {
        let undo = board.make_move(&mv);
        let cur_score = -negamax(board, depth - 1, i32::MIN, i32::MAX);
        board.unmake_move(&mv, &undo);

        if cur_score > best_score {
            best_score = cur_score;
            best_move = Some(*mv);
        }
    }

    let nodes = NODES.with(|n| n.get());
    (best_move, nodes)
}

fn negamax(board: &mut Board, depth: u16, mut alpha: i32, beta: i32) -> i32 {
    inc_nodes();

    if depth == 0 {
        let color_fac = if board.side_to_move() == Color::White {
            1
        } else {
            -1
        };
        return evaluate(board) * color_fac;
    }

    let mut max_eval = i32::MIN;

    for mv in board.gen_moves().as_slice() {
        let undo = board.make_move(&mv);
        let eval = -negamax(board, depth - 1, -beta, -alpha);
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

    max_eval
}
