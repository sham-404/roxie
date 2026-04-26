use crate::{board::Board, items::{Move, Rng}};

pub fn find_best_move(board: &mut Board) -> Move {
    let moves = board.gen_moves();

    let mut rng = Rng::new(29834827345);
    let idx = rng.gen_range(moves.len());
    moves[idx]
}
