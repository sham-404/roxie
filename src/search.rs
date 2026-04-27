use crate::{board::Board, items::{Move, MoveFlag, Rng}};

pub fn find_best_move(board: &mut Board) -> Move {
    let moves = board.gen_moves();

    let mut rng = Rng::new(29834827345);
    if moves.len() == 0 {
        return Move::new(64, 64, MoveFlag::Quiet);
    }
    
    let idx = rng.gen_range(moves.len());
    moves[idx]
}
