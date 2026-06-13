use std::sync::OnceLock;

use crate::{
    board::{Board, pop_lsb},
    r#const::{BLACK_PASSED_MASKS, KING_ATTACKS, KNIGHT_ATTACKS, WHITE_PASSED_MASKS},
    items::{Color, Piece},
    magics::{get_bishop_move_bits, get_rook_move_bits}, network::NETWORK,
};

// const SCORE: [i32; 5] = [100, 320, 330, 500, 900];

// Piece score wrt index
// 0 -> pawn, 1 -> knight, 2 -> bishop, 3 -> rook, 4 -> queen
pub const MG_VALUE: [i32; 6] = [82, 337, 365, 477, 1025, 0];
const EG_VALUE: [i32; 6] = [94, 281, 297, 512, 936, 0];

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
const EG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
     94, 100,  85,  67,  56,  53,  82,  84,
     32,  24,  13,   5,  -2,   4,  17,  17,
     13,   9,  -3,  -7,  -7,  -8,   3,  -1,
      4,   7,  -6,   1,   0,  -5,  -1,  -8,
     13,   8,   8,  10,  13,   0,   2,  -7,
      0,   0,   0,   0,   0,   0,   0,   0,
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
const EG_KNIGHT_TABLE: [i32; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
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
const EG_BISHOP_TABLE: [i32; 64] = [
    -14, -21, -11,  -8, -7,  -9, -17, -24,
     -8,  -4,   7, -12, -3, -13,  -4, -14,
      2,  -8,   0,  -1, -2,   6,   0,   4,
     -3,   9,  12,   9, 14,  10,   3,   2,
     -6,   3,  13,  19,  7,  10,  -3,  -9,
    -12,  -3,   8,  10, 13,   3,  -7, -15,
    -14, -18,  -7,  -1,  4,  -9, -15, -27,
    -23,  -9, -23,  -5, -9, -16,  -5, -17,
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
const EG_ROOK_TABLE: [i32; 64] = [
    13, 10, 18, 15, 12,  12,   8,   5,
    11, 13, 13, 11, -3,   3,   8,   3,
     7,  7,  7,  5,  4,  -3,  -5,  -3,
     4,  3, 13,  1,  2,   1,  -1,   2,
     3,  5,  8,  4, -5,  -6,  -8, -11,
    -4,  0, -5, -1, -7, -12,  -8, -16,
    -6, -6,  0,  2, -9,  -9, -11,  -3,
    -9,  2,  3, -1, -5, -13,   4, -20,
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
const EG_QUEEN_TABLE: [i32; 64] = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
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

#[rustfmt::skip]
const EG_KING_TABLE: [i32; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43
];

const MG_PESTO_TABLE: [[i32; 64]; 6] = [
    MG_PAWN_TABLE,
    MG_KNIGHT_TABLE,
    MG_BISHOP_TABLE,
    MG_ROOK_TABLE,
    MG_QUEEN_TABLE,
    MG_KING_TABLE,
];

const EG_PESTO_TABLE: [[i32; 64]; 6] = [
    EG_PAWN_TABLE,
    EG_KNIGHT_TABLE,
    EG_BISHOP_TABLE,
    EG_ROOK_TABLE,
    EG_QUEEN_TABLE,
    EG_KING_TABLE,
];

pub const GAME_PHASE_VAL: [i32; 12] = [0, 1, 1, 2, 4, 0, 0, 1, 1, 2, 4, 0];

pub static EG_TABLE: OnceLock<[[i32; 64]; 12]> = OnceLock::new();
pub static MG_TABLE: OnceLock<[[i32; 64]; 12]> = OnceLock::new();

#[inline(always)]
fn mirror(sq: usize) -> usize {
    sq ^ 56
}

pub fn init_pesto_table() {
    EG_TABLE.get_or_init(|| {
        let mut table = [[0i32; 64]; 12];
        for p_idx in 0..6 {
            for sq in 0..64 {
                table[p_idx][sq] = EG_VALUE[p_idx] + EG_PESTO_TABLE[p_idx][sq];
                table[p_idx + 6][sq] = EG_VALUE[p_idx] + EG_PESTO_TABLE[p_idx][mirror(sq)];
            }
        }
        table
    });

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

const KNIGHT_MOBILITY: [i32; 9] = [-20, -8, 0, 6, 12, 17, 21, 24, 26];
const BISHOP_MOBILITY: [i32; 14] = [-15, -6, 0, 5, 10, 15, 19, 23, 26, 28, 30, 31, 32, 33];
const ROOK_MOBILITY: [i32; 15] = [-10, -5, 0, 3, 6, 9, 12, 15, 18, 21, 23, 25, 27, 28, 29];
const _QUEEN_MOBILITY: [i32; 28] = [
    -15, -10, -5, 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
    30, 30, 30, 30,
];

const BISHOP_PAIR_BONUS: i32 = 30;
fn mobility_score(board: &Board) -> i32 {
    let all_occ = board.all_occ();
    let mut score = 0;

    let white_occ = board.occ(&Color::White);
    let black_occ = board.occ(&Color::Black);

    let white_pawn_attacks = board.gen_pawn_attack_map(Color::White);
    let black_pawn_attacks = board.gen_pawn_attack_map(Color::Black);

    let white_safe_sq = !white_occ & !black_pawn_attacks;
    let black_safe_sq = !black_occ & !white_pawn_attacks;

    //////// WHITE PIECE MOBILITY SCORING

    let mut w_knight = board.bb(Piece::WHITE | Piece::KNIGHT);
    while let Some(sq) = pop_lsb(&mut w_knight) {
        let move_count = (KNIGHT_ATTACKS[sq] & white_safe_sq).count_ones();
        score += KNIGHT_MOBILITY[move_count as usize];
    }

    let mut w_bishop = board.bb(Piece::WHITE | Piece::BISHOP);
    if w_bishop.count_ones() > 1 {
        score += BISHOP_PAIR_BONUS;
    }

    while let Some(sq) = pop_lsb(&mut w_bishop) {
        let move_count = (get_bishop_move_bits(sq, all_occ) & white_safe_sq).count_ones();
        score += BISHOP_MOBILITY[move_count as usize];
    }

    let mut w_rook = board.bb(Piece::WHITE | Piece::ROOK);
    while let Some(sq) = pop_lsb(&mut w_rook) {
        let move_count = (get_rook_move_bits(sq, all_occ) & white_safe_sq).count_ones();
        score += ROOK_MOBILITY[move_count as usize];
    }

    //////// BLACK PIECE MOBILITY SCORING

    let mut b_knight = board.bb(Piece::BLACK | Piece::KNIGHT);
    while let Some(sq) = pop_lsb(&mut b_knight) {
        let move_count = (KNIGHT_ATTACKS[sq] & black_safe_sq).count_ones();
        score -= KNIGHT_MOBILITY[move_count as usize];
    }

    let mut b_bishop = board.bb(Piece::BLACK | Piece::BISHOP);
    if b_bishop.count_ones() > 1 {
        score -= BISHOP_PAIR_BONUS;
    }

    while let Some(sq) = pop_lsb(&mut b_bishop) {
        let move_count = (get_bishop_move_bits(sq, all_occ) & black_safe_sq).count_ones();
        score -= BISHOP_MOBILITY[move_count as usize];
    }

    let mut b_rook = board.bb(Piece::BLACK | Piece::ROOK);
    while let Some(sq) = pop_lsb(&mut b_rook) {
        let move_count = (get_rook_move_bits(sq, all_occ) & black_safe_sq).count_ones();
        score -= ROOK_MOBILITY[move_count as usize];
    }

    score
}

const ADJ_FILE_MASKS: [u64; 8] = [
    0x0202020202020202,
    0x0505050505050505,
    0x0A0A0A0A0A0A0A0A,
    0x1414141414141414,
    0x2828282828282828,
    0x5050505050505050,
    0xA0A0A0A0A0A0A0A0,
    0x4040404040404040,
];
const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 20, 35, 60, 100, 0];
const DOUBLE_PAWN_PENALTY: i32 = 20;
const ISO_PAWN_PENALTY: i32 = 25;

fn pawn_struct_score(board: &Board) -> i32 {
    let white_pawns = board.bb(Piece::WHITE | Piece::PAWN);
    let black_pawns = board.bb(Piece::BLACK | Piece::PAWN);

    let mut score = 0;

    // Arrays to track how many pawns are on each file for the doubled penalty
    let mut w_file_counts = [0; 8];
    let mut b_file_counts = [0; 8];

    // EVALUATE WHITE PAWNS
    let mut w_bb = white_pawns;
    while let Some(sq) = pop_lsb(&mut w_bb) {
        let file = sq % 8;
        let rank = sq / 8;

        // Isolated Pawns
        if ADJ_FILE_MASKS[file] & white_pawns == 0 {
            score -= ISO_PAWN_PENALTY;
        }

        // Passed Pawns
        if black_pawns & WHITE_PASSED_MASKS[sq] == 0 {
            score += PASSED_PAWN_BONUS[rank];
        }

        // Track for doubled pawns
        w_file_counts[file] += 1;
    }

    // EVALUATE BLACK PAWNS
    let mut b_bb = black_pawns;
    while let Some(sq) = pop_lsb(&mut b_bb) {
        let file = sq % 8;
        let rank = sq / 8;

        // Isolated Pawns
        if ADJ_FILE_MASKS[file] & black_pawns == 0 {
            score += ISO_PAWN_PENALTY;
        }

        // Passed Pawns
        if white_pawns & BLACK_PASSED_MASKS[sq] == 0 {
            score -= PASSED_PAWN_BONUS[7 - rank];
        }

        // Track for doubled pawns
        b_file_counts[file] += 1;
    }

    // APPLYING DOUBLED PAWN PENALTIES
    for i in 0..8 {
        if w_file_counts[i] > 1 {
            score -= (w_file_counts[i] - 1) * DOUBLE_PAWN_PENALTY;
        }
        if b_file_counts[i] > 1 {
            score += (b_file_counts[i] - 1) * DOUBLE_PAWN_PENALTY;
        }
    }

    score
}

const MISSING_PAWN_SHIELD_PENALTY: i32 = 35;
const ADVANCED_PAWN_SHIELD_PENALTY: i32 = 10;
const NOT_CASTLED_YET_PENALTY: i32 = 5;

const WHITE_SAFE_RANKS: u64 = 0x0000000000FFFF00; // Ranks 2 and 3
const BLACK_SAFE_RANKS: u64 = 0x00FFFF0000000000; // Ranks 7 and 6

const FILE_MASKS: [u64; 8] = [
    0x0101010101010101, 0x0202020202020202, 0x0404040404040404, 0x0808080808080808,
    0x1010101010101010, 0x2020202020202020, 0x4040404040404040, 0x8080808080808080,
];

const KNIGHT_ATTACK_VAL: i32 = 20;
const BISHOP_ATTACK_VAL: i32 = 20;
const ROOK_ATTACK_VAL: i32 = 40;
const QUEEN_ATTACK_VAL: i32 = 80;

const ATTACK_WEIGHT_TABLE: [i32; 8] = [0, 0, 50, 75, 88, 94, 97, 99];

#[rustfmt::skip]
const _SAFETY_TABLE: [i32; 100] = [
       0,  0,   1,   2,   3,   5,   7,   9,  12,  15,
      18,  22,  26,  30,  35,  39,  44,  50,  56,  62,
      68,  75,  82,  85,  89,  97, 105, 113, 122, 131,
     140, 150, 169, 180, 191, 202, 213, 225, 237, 248,
     260, 272, 283, 295, 307, 319, 330, 342, 354, 366,
     377, 389, 401, 412, 424, 436, 448, 459, 471, 483,
     494, 500, 500, 500, 500, 500, 500, 500, 500, 500,
     500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
     500, 500, 500, 500, 500, 500, 500, 500, 500, 500,
     500, 500, 500, 500, 500, 500, 500, 500, 500, 500
];

const fn compute_king_zones(is_white: bool) -> [u64; 64] {
    let mut zones = [0; 64];
    let mut sq = 0;

    while sq < 64 {
        let ring = KING_ATTACKS[sq];

        if is_white {
            zones[sq] = ring | (ring << 8); // Shift UP for White
        } else {
            zones[sq] = ring | (ring >> 8); // Shift DOWN for Black
        }
        sq += 1;
    }
    zones
}

pub const WHITE_KING_ZONES: [u64; 64] = compute_king_zones(true);
pub const BLACK_KING_ZONES: [u64; 64] = compute_king_zones(false);

fn king_safety_score(board: &Board) -> i32 {
    let phase = board.get_game_phase();
    let king_safety_start_phase = 6;
    
    // Early exit
    if phase <= king_safety_start_phase {
        return 0;
    }

    let mut score = 0;
    let all_occ = board.all_occ();

    let w_pawns = board.bb(Piece::WHITE | Piece::PAWN);
    let b_pawns = board.bb(Piece::BLACK | Piece::PAWN);

    let w_king = board.bb(Piece::WHITE | Piece::KING);
    let b_king = board.bb(Piece::BLACK | Piece::KING);

    let w_king_sq = w_king.trailing_zeros() as usize;
    let b_king_sq = b_king.trailing_zeros() as usize;

    let w_king_file = w_king_sq % 8;
    let b_king_file = b_king_sq % 8;

    //////// DEFENSE: PAWN SHIELD EVALUATION

    // White Shield
    if w_king_file < 3 || w_king_file > 4 {
        let start_file = w_king_file.saturating_sub(1);
        let end_file = (w_king_file + 1).min(7);

        for file in start_file..=end_file {
            let file_pawns = w_pawns & FILE_MASKS[file];

            if file_pawns == 0 {
                score -= MISSING_PAWN_SHIELD_PENALTY;
            } else if file_pawns & WHITE_SAFE_RANKS == 0 {
                score -= ADVANCED_PAWN_SHIELD_PENALTY;
            }
        }
    } else {
        score -= NOT_CASTLED_YET_PENALTY;
    }

    // Black Shield
    if b_king_file < 3 || b_king_file > 4 {
        let start_file = b_king_file.saturating_sub(1);
        let end_file = (b_king_file + 1).min(7);

        for file in start_file..=end_file {
            let file_pawns = b_pawns & FILE_MASKS[file];

            if file_pawns == 0 {
                score += MISSING_PAWN_SHIELD_PENALTY;
            } else if file_pawns & BLACK_SAFE_RANKS == 0 {
                score += ADVANCED_PAWN_SHIELD_PENALTY;
            }
        }
    } else {
        score += NOT_CASTLED_YET_PENALTY;
    }

    //////// OFFENSE: TOGA ATTACK EVALUATION

    let w_king_zone = WHITE_KING_ZONES[w_king_sq];
    let b_king_zone = BLACK_KING_ZONES[b_king_sq];

    // Black Attacks on White King Zone 
    let mut b_attacking_pieces = 0;
    let mut b_value_of_attacks = 0;

    let mut b_knight = board.bb(Piece::BLACK | Piece::KNIGHT);
    while let Some(sq) = pop_lsb(&mut b_knight) {
        let attacked_squares = (KNIGHT_ATTACKS[sq] & w_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            b_attacking_pieces += 1;
            b_value_of_attacks += attacked_squares * KNIGHT_ATTACK_VAL;
        }
    }

    let mut b_bishop = board.bb(Piece::BLACK | Piece::BISHOP);
    while let Some(sq) = pop_lsb(&mut b_bishop) {
        let attacked_squares = (get_bishop_move_bits(sq, all_occ) & w_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            b_attacking_pieces += 1;
            b_value_of_attacks += attacked_squares * BISHOP_ATTACK_VAL;
        }
    }

    let mut b_rook = board.bb(Piece::BLACK | Piece::ROOK);
    while let Some(sq) = pop_lsb(&mut b_rook) {
        let attacked_squares = (get_rook_move_bits(sq, all_occ) & w_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            b_attacking_pieces += 1;
            b_value_of_attacks += attacked_squares * ROOK_ATTACK_VAL;
        }
    }

    let mut b_queen = board.bb(Piece::BLACK | Piece::QUEEN);
    while let Some(sq) = pop_lsb(&mut b_queen) {
        let queen_attacks = get_bishop_move_bits(sq, all_occ) | get_rook_move_bits(sq, all_occ);
        let attacked_squares = (queen_attacks & w_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            b_attacking_pieces += 1;
            b_value_of_attacks += attacked_squares * QUEEN_ATTACK_VAL;
        }
    }

    if b_attacking_pieces > 1 {
        let b_weight_index = (b_attacking_pieces as usize).min(7);
        let white_danger = (b_value_of_attacks * ATTACK_WEIGHT_TABLE[b_weight_index]) / 100;
        score -= white_danger
    }

    // White Attacks on Black King Zone 
    let mut w_attacking_pieces = 0;
    let mut w_value_of_attacks = 0;

    let mut w_knight = board.bb(Piece::WHITE | Piece::KNIGHT);
    while let Some(sq) = pop_lsb(&mut w_knight) {
        let attacked_squares = (KNIGHT_ATTACKS[sq] & b_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            w_attacking_pieces += 1;
            w_value_of_attacks += attacked_squares * KNIGHT_ATTACK_VAL;
        }
    }

    let mut w_bishop = board.bb(Piece::WHITE | Piece::BISHOP);
    while let Some(sq) = pop_lsb(&mut w_bishop) {
        let attacked_squares = (get_bishop_move_bits(sq, all_occ) & b_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            w_attacking_pieces += 1;
            w_value_of_attacks += attacked_squares * BISHOP_ATTACK_VAL;
        }
    }

    let mut w_rook = board.bb(Piece::WHITE | Piece::ROOK);
    while let Some(sq) = pop_lsb(&mut w_rook) {
        let attacked_squares = (get_rook_move_bits(sq, all_occ) & b_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            w_attacking_pieces += 1;
            w_value_of_attacks += attacked_squares * ROOK_ATTACK_VAL;
        }
    }

    let mut w_queen = board.bb(Piece::WHITE | Piece::QUEEN);
    while let Some(sq) = pop_lsb(&mut w_queen) {
        let queen_attacks = get_bishop_move_bits(sq, all_occ) | get_rook_move_bits(sq, all_occ);
        let attacked_squares = (queen_attacks & b_king_zone).count_ones() as i32;
        if attacked_squares > 0 {
            w_attacking_pieces += 1;
            w_value_of_attacks += attacked_squares * QUEEN_ATTACK_VAL;
        }
    }

    if w_attacking_pieces > 1 {
        let w_weight_index = (w_attacking_pieces as usize).min(7);
        let black_danger = (w_value_of_attacks * ATTACK_WEIGHT_TABLE[w_weight_index]) / 100;
        score += black_danger
    }

    (score * (phase - king_safety_start_phase)) / (24 - king_safety_start_phase)
}

const TEMPO_BONUS: i32 = 10;
pub fn evaluate(board: &Board, acc: &[f32]) -> i32 {
    let mut score = 0;

    if let Some(nn) = NETWORK.get() {
        return nn.evaluate_with_acc(acc);
    }

    score += board.get_pesto_score();
    score += mobility_score(board);
    score += pawn_struct_score(board);
    score += king_safety_score(board);

    (score + TEMPO_BONUS) * board.side_to_move().fac()
}
