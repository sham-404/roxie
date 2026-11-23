use std::sync::OnceLock;

use crate::{
    board::Board,
    r#const::{BLACK, MAX_PLY, WHITE},
    engine::Engine,
    items::Color,
    network::{HL1, NETWORK},
    search::MATE,
};

// const SCORE: [i32; 5] = [100, 320, 330, 500, 900];

// Piece score wrt index
// 0 -> pawn, 1 -> knight, 2 -> bishop, 3 -> rook, 4 -> queen
pub const MG_VALUE: [i32; 6] = [82, 337, 365, 477, 1025, 0];

#[rustfmt::skip]
const MG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,  0,   0,
     98, 134,  61,  95,  68, 126, 34, -11,
     -6,   7,  26,  31,  65,  56, 25, -20,
    -14,  13,   6,  21,  23,  12, 17, -23,
    -27,  -2,  -5,  12,  17,   6, 10, -25,
    -26,  -4,  -4, -10,   3,   3, 33, -12,
    -35,  -1, -20, -23, -15,  24, 38, -22,
      0,   0,   0,   0,   0,   0,  0,   0,
];

#[rustfmt::skip]
const MG_KNIGHT_TABLE: [i32; 64] = [
    -167, -89, -34, -49,  61, -97, -15, -107,
     -73, -41,  72,  36,  23,  62,   7,  -17,
     -47,  60,  37,  65,  84, 129,  73,   44,
      -9,  17,  19,  53,  37,  69,  18,   22,
     -13,   4,  16,  13,  28,  19,  21,   -8,
     -23,  -9,  12,  10,  19,  17,  25,  -16,
     -29, -53, -12,  -3,  -1,  18, -14,  -19,
    -105, -21, -58, -33, -17, -28, -19,  -23,
];

#[rustfmt::skip]
const MG_BISHOP_TABLE: [i32; 64] = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];

#[rustfmt::skip]
const MG_ROOK_TABLE: [i32; 64] = [
     32,  42,  32,  51, 63,  9,  31,  43,
     27,  32,  58,  62, 80, 67,  26,  44,
     -5,  19,  26,  36, 17, 45,  61,  16,
    -24, -11,   7,  26, 24, 35,  -8, -20,
    -36, -26, -12,  -1,  9, -7,   6, -23,
    -45, -25, -16, -17,  3,  0,  -5, -33,
    -44, -16, -20,  -9, -1, 11,  -6, -71,
    -19, -13,   1,  17, 16,  7, -37, -26,
];

#[rustfmt::skip]
const MG_QUEEN_TABLE: [i32; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50,
];

#[rustfmt::skip]
const MG_KING_TABLE: [i32; 64] = [
    -65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];

const MG_PESTO_TABLE: [[i32; 64]; 6] = [
    MG_PAWN_TABLE,
    MG_KNIGHT_TABLE,
    MG_BISHOP_TABLE,
    MG_ROOK_TABLE,
    MG_QUEEN_TABLE,
    MG_KING_TABLE,
];

pub const GAME_PHASE_VAL: [i32; 12] = [0, 1, 1, 2, 4, 0, 0, 1, 1, 2, 4, 0];

pub static MG_TABLE: OnceLock<[[i32; 64]; 12]> = OnceLock::new();

#[inline(always)]
pub fn mirror(sq: usize) -> usize {
    sq ^ 56
}

pub fn init_pesto_table() {
    MG_TABLE.get_or_init(|| {
        let mut table = [[0i32; 64]; 12];
        for p_idx in 0..6 {
            for sq in 0..64 {
                // mirroring for white because the PeSTO table constants are in blacks perspective
                table[p_idx][sq] = MG_VALUE[p_idx] + MG_PESTO_TABLE[p_idx][mirror(sq)];
                table[p_idx + 6][sq] = MG_VALUE[p_idx] + MG_PESTO_TABLE[p_idx][sq];
            }
        }
        table
    });
}

fn scaled_score_for_50_mv_rule(board: &Board, score: i32) -> i32 {
    if score.abs() > MATE as i32 - MAX_PLY as i32 {
        return score;
    }

    let halfmoves = board.halfmove_clock();

    // scaling done when halfmove goes above 70
    if halfmoves > 70 {
        let remaining = 100 - halfmoves as i32;
        let scale = (remaining * 100) / 30;

        return score * scale / 100;
    }
    score
}

impl Engine {
    pub fn evaluate(&mut self, ply: usize) -> i16 {
        let nn = NETWORK
            .get()
            .expect("Network is not initialized for some bizzare reasons");
        let mut acc_for_eval = [0; HL1 * 2];
        let acc = self.accumulators.get(ply);
        match self.board.side_to_move() {
            Color::White => {
                acc_for_eval[..HL1].copy_from_slice(&acc[WHITE]);
                acc_for_eval[HL1..].copy_from_slice(&acc[BLACK]);
            }
            Color::Black => {
                acc_for_eval[..HL1].copy_from_slice(&acc[BLACK]);
                acc_for_eval[HL1..].copy_from_slice(&acc[WHITE]);
            }
        };

        let score = nn.eval_with_acc(&mut self.eval_buf, &acc_for_eval) as i32;
        return scaled_score_for_50_mv_rule(&self.board, score) as i16;
    }
}
