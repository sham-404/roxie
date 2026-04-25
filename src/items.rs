use crate::r#const::*;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    WP,
    WN,
    WB,
    WR,
    WQ,
    WK,

    BP,
    BN,
    BB,
    BR,
    BQ,
    BK,
}

#[allow(dead_code)]
impl Piece {
    pub fn piece_to_char(piece: Piece) -> char {
        match piece {
            Piece::WP => 'P',
            Piece::WN => 'N',
            Piece::WB => 'B',
            Piece::WR => 'R',
            Piece::WQ => 'Q',
            Piece::WK => 'K',

            Piece::BP => 'p',
            Piece::BN => 'n',
            Piece::BB => 'b',
            Piece::BR => 'r',
            Piece::BQ => 'q',
            Piece::BK => 'k',
        }
    }

    pub fn piece_to_glyph(piece: Piece) -> &'static str {
        match piece {
            Piece::WP => "♟",
            Piece::WN => "♞",
            Piece::WB => "♝",
            Piece::WR => "♜",
            Piece::WQ => "♛",
            Piece::WK => "♚",

            Piece::BP => "♙",
            Piece::BN => "♘",
            Piece::BB => "♗",
            Piece::BR => "♖",
            Piece::BQ => "♕",
            Piece::BK => "♔",
        }
    }

    pub fn from_val(val: usize) -> Self {
        match val {
            0 => Piece::WP,
            1 => Piece::WN,
            2 => Piece::WB,
            3 => Piece::WR,
            4 => Piece::WQ,
            5 => Piece::WK,

            6 => Piece::BP,
            7 => Piece::BN,
            8 => Piece::BB,
            9 => Piece::BR,
            10 => Piece::BQ,
            11 => Piece::BK,
            _ => unreachable!(),
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
}

#[derive(Debug)]
pub struct Undo {
    pub captured: Option<Piece>,
    pub prev_en_passant_sq: Option<u8>,
    pub prev_castling_rights: CastlingRights,
}

impl Undo {
    pub fn new(captured: Option<Piece>, castling: CastlingRights, ensq: Option<u8>) -> Self {
        Self {
            captured,
            prev_en_passant_sq: ensq,
            prev_castling_rights: castling,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
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
        Self(WK | WQ | BK | BQ)
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
