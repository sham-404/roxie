use crate::board::Board;
use crate::r#const::*;

pub type PieceInfo = u8;

pub struct Piece;

impl Piece {
    // Piece Types (Bits 0-2)
    pub const NONE: PieceInfo = 0;
    pub const PAWN: PieceInfo = 1;
    pub const KNIGHT: PieceInfo = 2;
    pub const BISHOP: PieceInfo = 3;
    pub const ROOK: PieceInfo = 4;
    pub const QUEEN: PieceInfo = 5;
    pub const KING: PieceInfo = 6;

    // Colors (Bits 3-4)
    pub const WHITE: PieceInfo = 8;
    pub const BLACK: PieceInfo = 16;

    // Masks
    const TYPE_MASK: u8 = 0b00111;
    const COLOR_MASK: u8 = 0b11000;

    #[inline(always)]
    pub fn get_type(p: PieceInfo) -> u8 {
        p & Self::TYPE_MASK
    }

    #[inline(always)]
    pub fn get_color(p: PieceInfo) -> u8 {
        p & Self::COLOR_MASK
    }

    #[inline(always)]
    pub fn from_idx(idx: usize) -> PieceInfo {
        // idx 0-5 -> White Pawn to King
        // idx 6-11 -> Black Pawn to King
        let color = if idx < 6 { Self::WHITE } else { Self::BLACK };
        let piece_type = (idx % 6) as u8 + 1;
        color | piece_type
    }

    // Indexes the pieces from 1 t0 11 for bitboards
    pub const fn to_idx(p: PieceInfo) -> usize {
        // This maps:
        // White (P, N, B, R, Q, K) -> 0, 1, 2, 3, 4, 5
        // Black (P, N, B, R, Q, K) -> 6, 7, 8, 9, 10, 11
        let type_idx = (p & 0b111) as usize - 1;
        let color_idx = if (p & 16) != 0 { 6 } else { 0 };
        type_idx + color_idx
    }

    pub fn enemy(color: PieceInfo) -> PieceInfo {
        debug_assert!(
            color == color & Self::COLOR_MASK,
            "ts is not even a color dawg"
        );
        color ^ 0b11000
    }

    pub fn to_char(p: PieceInfo) -> char {
        let t = Self::get_type(p);
        let mut c = match t {
            Self::PAWN => 'P',
            Self::KNIGHT => 'N',
            Self::BISHOP => 'B',
            Self::ROOK => 'R',
            Self::QUEEN => 'Q',
            Self::KING => 'K',
            _ => return ' ',
        };
        if Self::get_color(p) == Self::BLACK {
            c = c.to_ascii_lowercase();
        }
        c
    }

    pub fn from_char(c: char) -> Option<PieceInfo> {
        let color = if c.is_uppercase() {
            Self::WHITE
        } else {
            Self::BLACK
        };
        let t = match c.to_ascii_uppercase() {
            'P' => Self::PAWN,
            'N' => Self::KNIGHT,
            'B' => Self::BISHOP,
            'R' => Self::ROOK,
            'Q' => Self::QUEEN,
            'K' => Self::KING,
            _ => return None,
        };
        Some(color | t)
    }

    pub fn to_glyph(p: PieceInfo) -> &'static str {
        match p {
            0b01001 => "♟", // White Pawn
            0b01010 => "♞", // White Knight
            0b01011 => "♝", // White Bishop
            0b01100 => "♜", // White Rook
            0b01101 => "♛", // White Queen
            0b01110 => "♚", // White King

            0b10001 => "♙", // Black Pawn (inverted for terminal contrast usually)
            0b10010 => "♘",
            0b10011 => "♗",
            0b10100 => "♖",
            0b10101 => "♕",
            0b10110 => "♔",
            _ => " ",
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MoveFlag {
    Quiet = 0b0000,
    DoublePush = 0b0001,
    KingCastle = 0b0010,
    QueenCastle = 0b0011,

    Capture = 0b1000,
    EnPassant = 0b1010,

    PromoKnight = 0b0100,
    PromoBishop = 0b0101,
    PromoRook = 0b0110,
    PromoQueen = 0b0111,

    PromoCapKnight = 0b1100,
    PromoCapBishop = 0b1101,
    PromoCapRook = 0b1110,
    PromoCapQueen = 0b1111,
}

use crate::r#const::SQ_TO_COORD;
#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub from: usize,
    pub to: usize,
    pub flag: MoveFlag,
}

impl Move {
    pub fn new(from: usize, to: usize, flag: MoveFlag) -> Self {
        Self { from, to, flag }
    }

    pub fn to_coord(&self) -> String {
        let mut coord = format!("{}{}", SQ_TO_COORD[self.from], SQ_TO_COORD[self.to]);

        let promo = match self.flag {
            MoveFlag::PromoQueen | MoveFlag::PromoCapQueen => Some('q'),
            MoveFlag::PromoKnight | MoveFlag::PromoCapKnight => Some('n'),
            MoveFlag::PromoRook | MoveFlag::PromoCapRook => Some('r'),
            MoveFlag::PromoBishop | MoveFlag::PromoCapBishop => Some('b'),
            _ => None,
        };

        if let Some(c) = promo {
            coord.push(c);
        }

        coord
    }

    pub fn from_uci(mv_str: &str, board: &mut Board) -> Move {
        let moves = board.gen_moves();

        for mv in moves {
            if mv.to_coord() == mv_str {
                return mv;
            }
        }

        panic!("Invalid move: {}", mv_str);
    }
}

#[derive(Debug)]
pub struct Undo {
    pub captured: PieceInfo,
    pub prev_en_passant_sq: Option<u8>,
    pub prev_castling_rights: CastlingRights,
}

impl Undo {
    pub fn new(captured: PieceInfo, castling: CastlingRights, ensq: Option<u8>) -> Self {
        Self {
            captured,
            prev_en_passant_sq: ensq,
            prev_castling_rights: castling,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opponent(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn white_kingside(self) -> bool {
        self.0 & WK != 0
    }

    pub fn white_queenside(self) -> bool {
        self.0 & WQ != 0
    }

    pub fn black_kingside(self) -> bool {
        self.0 & BK != 0
    }

    pub fn black_queenside(self) -> bool {
        self.0 & BQ != 0
    }

    // remove rights
    pub fn remove(&mut self, mask: u8) {
        self.0 &= !mask;
    }

    // add rights
    pub fn add(&mut self, mask: u8) {
        self.0 |= mask;
    }
}

//// Helpers

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn gen_range(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
}
