use crate::{board::Board, items::{Move, Rng}};

pub fn find_best_move(board: &mut Board) -> Option<Move> {
    let moves = board.gen_moves();

    let mut rng = Rng::new(29834827345);
    if moves.len() == 0 {
        return None;
    }
    
    let idx = rng.gen_range(moves.len());
    Some(moves[idx])
}
