use crate::board::Board;
use crate::items::{Piece, PieceInfo};

pub const PRIME_WHITE_QUEEN: u64 = 11811845319353239651;
pub const PRIME_WHITE_ROOK: u64 = 10979190538029446137;
pub const PRIME_WHITE_BISHOP: u64 = 12311744257139811149;
pub const PRIME_WHITE_KNIGHT: u64 = 15202887380319082783;
pub const PRIME_WHITE_PAWN: u64 = 17008651141875982339;
pub const PRIME_BLACK_QUEEN: u64 = 15484752644942473553;
pub const PRIME_BLACK_ROOK: u64 = 18264461213049635989;
pub const PRIME_BLACK_BISHOP: u64 = 15394650811035483107;
pub const PRIME_BLACK_KNIGHT: u64 = 13469005675588064321;
pub const PRIME_BLACK_PAWN: u64 = 11695583624105689831;

// Material keys only cover the non-king piece types, indexed
// PAWN, KNIGHT, BISHOP, ROOK, QUEEN. Kings are excluded, which is why a KvK
// position has a material key of 0.
const PRIME_WHITE: [u64; 5] = [
    PRIME_WHITE_PAWN,
    PRIME_WHITE_KNIGHT,
    PRIME_WHITE_BISHOP,
    PRIME_WHITE_ROOK,
    PRIME_WHITE_QUEEN,
];
const PRIME_BLACK: [u64; 5] = [
    PRIME_BLACK_PAWN,
    PRIME_BLACK_KNIGHT,
    PRIME_BLACK_BISHOP,
    PRIME_BLACK_ROOK,
    PRIME_BLACK_QUEEN,
];

fn key_from_counts(white: &[u64; 5], black: &[u64; 5]) -> u64 {
    let mut key = 0u64;
    for i in 0..5 {
        key = key
            .wrapping_add(white[i].wrapping_mul(PRIME_WHITE[i]))
            .wrapping_add(black[i].wrapping_mul(PRIME_BLACK[i]));
    }
    key
}

pub fn calc_key(board: &Board, mirror: bool) -> u64 {
    let mut white = [0u64; 5];
    let mut black = [0u64; 5];

    for i in 0..5 {
        let pt = (i + 1) as PieceInfo;
        white[i] = board.bb(Piece::WHITE | pt).count_ones() as u64;
        black[i] = board.bb(Piece::BLACK | pt).count_ones() as u64;
    }

    if mirror {
        std::mem::swap(&mut white, &mut black);
    }

    key_from_counts(&white, &black)
}

// Material key of a tablebase name such as "KQvKP". With `mirror` set, the
// colors of the two sides are exchanged, which yields the key the opposite
// color arrangement of the very same material produces. This mirrors
// Fathom's calc_key_from_pcs().
pub fn calc_key_from_name(name: &str, mirror: bool) -> u64 {
    let mut white = [0u64; 5];
    let mut black = [0u64; 5];

    let mut color_black = false;
    for c in name.chars() {
        if c == 'v' {
            color_black = true;
            continue;
        }
        let i = match c {
            'P' => 0,
            'N' => 1,
            'B' => 2,
            'R' => 3,
            'Q' => 4,
            _ => continue,
        };
        if color_black {
            black[i] += 1;
        } else {
            white[i] += 1;
        }
    }

    if mirror {
        std::mem::swap(&mut white, &mut black);
    }

    key_from_counts(&white, &black)
}
