use crate::board::{Board, mask, pop_lsb};
use crate::r#const::{BLACK_PAWN_ATTACKS, KING_ATTACKS, KNIGHT_ATTACKS, WHITE_PAWN_ATTACKS};
use crate::items::{Color, Piece, PieceInfo};
use crate::magics::{get_bishop_move_bits, get_rook_move_bits};

pub const TB_PAWN: u8 = 1;
pub const TB_KNIGHT: u8 = 2;
pub const TB_BISHOP: u8 = 3;
pub const TB_ROOK: u8 = 4;
pub const TB_QUEEN: u8 = 5;
pub const TB_KING: u8 = 6;

pub const TB_WPAWN: u8 = TB_PAWN;
pub const TB_BPAWN: u8 = TB_PAWN | 8;

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

pub type TbMove = u16;

pub const TB_PROMOTES_NONE: u16 = 0;
pub const TB_PROMOTES_QUEEN: u16 = 1;
pub const TB_PROMOTES_KNIGHT: u16 = 2;
pub const TB_PROMOTES_ROOK: u16 = 3;
pub const TB_PROMOTES_BISHOP: u16 = 4;

#[inline(always)]
pub const fn make_tb_move(promote: u16, from: u8, to: u8) -> TbMove {
    ((promote & 0x7) << 12) | (((from as u16) & 0x3F) << 6) | ((to as u16) & 0x3F)
}

#[inline(always)]
pub const fn tb_move_from(m: TbMove) -> u8 {
    ((m >> 6) & 0x3F) as u8
}

#[inline(always)]
pub const fn tb_move_to(m: TbMove) -> u8 {
    (m & 0x3F) as u8
}

#[inline(always)]
pub const fn tb_move_promotes(m: TbMove) -> u16 {
    (m >> 12) & 0x7
}

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

fn add_tb_move(moves: &mut Vec<TbMove>, promotes: bool, from: u8, to: u8) {
    if !promotes {
        moves.push(make_tb_move(TB_PROMOTES_NONE, from, to));
    } else {
        moves.push(make_tb_move(TB_PROMOTES_QUEEN, from, to));
        moves.push(make_tb_move(TB_PROMOTES_KNIGHT, from, to));
        moves.push(make_tb_move(TB_PROMOTES_ROOK, from, to));
        moves.push(make_tb_move(TB_PROMOTES_BISHOP, from, to));
    }
}

pub fn gen_tb_captures(board: &Board, moves: &mut Vec<TbMove>) {
    let us_color = board.side_to_move();
    let opp_color = us_color.opponent();

    let them = board.occ(&opp_color);
    let all_occ = board.all_occ();

    let (c_bit, pawn_attacks, p_rank) = match us_color {
        Color::White => (Piece::WHITE, WHITE_PAWN_ATTACKS, 7),
        Color::Black => (Piece::BLACK, BLACK_PAWN_ATTACKS, 0),
    };

    let mut king_bb = board.bb(c_bit | Piece::KING);
    if let Some(from) = pop_lsb(&mut king_bb) {
        let mut atk = KING_ATTACKS[from] & them;
        while let Some(to) = pop_lsb(&mut atk) {
            add_tb_move(moves, false, from as u8, to as u8);
        }
    }

    let mut queen_bb = board.bb(c_bit | Piece::QUEEN);
    while let Some(from) = pop_lsb(&mut queen_bb) {
        let mut atk =
            (get_rook_move_bits(from, all_occ) | get_bishop_move_bits(from, all_occ)) & them;
        while let Some(to) = pop_lsb(&mut atk) {
            add_tb_move(moves, false, from as u8, to as u8);
        }
    }

    let mut rook_bb = board.bb(c_bit | Piece::ROOK);
    while let Some(from) = pop_lsb(&mut rook_bb) {
        let mut atk = get_rook_move_bits(from, all_occ) & them;
        while let Some(to) = pop_lsb(&mut atk) {
            add_tb_move(moves, false, from as u8, to as u8);
        }
    }

    let mut bishop_bb = board.bb(c_bit | Piece::BISHOP);
    while let Some(from) = pop_lsb(&mut bishop_bb) {
        let mut atk = get_bishop_move_bits(from, all_occ) & them;
        while let Some(to) = pop_lsb(&mut atk) {
            add_tb_move(moves, false, from as u8, to as u8);
        }
    }

    let mut knight_bb = board.bb(c_bit | Piece::KNIGHT);
    while let Some(from) = pop_lsb(&mut knight_bb) {
        let mut atk = KNIGHT_ATTACKS[from] & them;
        while let Some(to) = pop_lsb(&mut atk) {
            add_tb_move(moves, false, from as u8, to as u8);
        }
    }

    let mut pawn_bb = board.bb(c_bit | Piece::PAWN);
    while let Some(from) = pop_lsb(&mut pawn_bb) {
        let mut atk = pawn_attacks[from] & them;
        while let Some(to) = pop_lsb(&mut atk) {
            add_tb_move(moves, (to / 8) == p_rank, from as u8, to as u8);
        }

        if let Some(ep_sq) = board.get_en_passant() {
            if (pawn_attacks[from] & mask(ep_sq as usize)) != 0 {
                add_tb_move(moves, false, from as u8, ep_sq);
            }
        }
    }
}
