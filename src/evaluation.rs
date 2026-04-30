use crate::{board::Board, items::Piece};

// Piece score wrt index 
// 0 -> pawn, 1 -> knight, 2 -> bishop, 3 -> rook, 4 -> queen
const SCORE: [i32; 5] = [100, 320, 330, 500, 900];

pub fn evaluate(board: &Board) -> i32 {
    let mut score = 0;

    score += material_score(board);

    score
}

fn material_score(board: &Board) -> i32 {
    let mut score = 0;

    for type_idx in 0..5 {
        let white_piece = Piece::from_idx(type_idx);
        let black_piece = Piece::from_idx(type_idx + 6);

        score += board.bb(white_piece).count_ones() as i32 * SCORE[type_idx];
        score -= board.bb(black_piece).count_ones() as i32 * SCORE[type_idx];
    }

    score
}
