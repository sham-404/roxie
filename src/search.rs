use crate::{
    board::Board,
    evaluation::evaluate,
    items::{Color, Move},
};

pub fn find_best_move(board: &mut Board, depth: u16) -> Option<Move> {
    let mut best_move: Option<Move> = None;
    let mut best_score = i32::MIN;

    let moves = board.gen_moves();

    for mv in moves {
        let undo = board.make_move(&mv);
        let cur_score = -negamax(board, depth - 1);
        board.unmake_move(&mv, &undo);

        if cur_score > best_score {
            best_move = Some(mv);
            best_score = cur_score
        }
    }

    best_move
}

fn negamax(board: &mut Board, depth: u16) -> i32 {
    if depth == 0 {
        let color_fac = if board.side_to_move() == Color::White {
            1
        } else {
            -1
        };
        return evaluate(board) * color_fac;
    }

    let mut max_eval = i32::MIN;
    let moves = board.gen_moves();

    if moves.is_empty() {
        return if board.in_check() { -30000 } else { 0 };
    }

    for mv in moves {
        let undo = board.make_move(&mv);
        let eval = -negamax(board, depth - 1);
        board.unmake_move(&mv, &undo);

        if eval > max_eval {
            max_eval = eval;
        }
    }

    max_eval
}
