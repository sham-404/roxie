#[repr(u8)]
#[derive(Copy, Clone, Debug)]
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

    pub fn from_val(val: usize) -> Self {
        let piece = match val {
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
        };

        piece
    }
}

#[allow(dead_code)]
#[repr(u8)]
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

#[allow(dead_code)]
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

#[repr(u8)]
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
