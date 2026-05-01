use crate::board::Board;
use crate::r#const::*;

pub type PieceInfo = u8;
pub type FlagInfo = u8;

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
    pub fn get_color_idx(p: PieceInfo) -> usize {
        if p & Self::COLOR_MASK == Piece::WHITE {
            WHITE
        } else {
            BLACK
        }
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
    #[inline(always)]
    pub const fn to_idx(p: PieceInfo) -> usize {
        // This maps:
        // White (P, N, B, R, Q, K) -> 0, 1, 2, 3, 4, 5
        // Black (P, N, B, R, Q, K) -> 6, 7, 8, 9, 10, 11
        let type_idx = (p & 0b111) as usize - 1;
        let color_idx = if (p & 16) != 0 { 6 } else { 0 };
        type_idx + color_idx
    }

    #[inline(always)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveFlag(pub FlagInfo);
//     0          0          00           -> 4 bit encoding
// promotion   capture  promo_pieces or   -> their representation
//                      special_moves

impl MoveFlag {
    pub const QUIET: MoveFlag = MoveFlag(0b0000);
    pub const DOUBLE_PUSH: MoveFlag = MoveFlag(0b0001);
    pub const KING_CASTLE: MoveFlag = MoveFlag(0b0010);
    pub const QUEEN_CASTLE: MoveFlag = MoveFlag(0b0011);

    pub const CAPTURE: MoveFlag = MoveFlag(0b1000);
    pub const EN_PASSANT: MoveFlag = MoveFlag(0b1010);

    // Check masks
    pub const CAPTURE_BIT: FlagInfo = 0b1000;
    pub const PROMO_BIT: FlagInfo = 0b0100;
    pub const PIECE_BIT: FlagInfo = 0b0011;

    // Raw bits for the match in to_coord and make_move
    pub const KNIGHT: FlagInfo = 0b00;
    pub const BISHOP: FlagInfo = 0b01;
    pub const ROOK: FlagInfo = 0b10;
    pub const QUEEN: FlagInfo = 0b11;

    // Promotion variants
    pub const PROMO_KNIGHT: MoveFlag = MoveFlag(0b0100);
    pub const PROMO_BISHOP: MoveFlag = MoveFlag(0b0101);
    pub const PROMO_ROOK: MoveFlag = MoveFlag(0b0110);
    pub const PROMO_QUEEN: MoveFlag = MoveFlag(0b0111);

    pub const PROMO_CAP_KNIGHT: MoveFlag = MoveFlag(0b1100);
    pub const PROMO_CAP_BISHOP: MoveFlag = MoveFlag(0b1101);
    pub const PROMO_CAP_ROOK: MoveFlag = MoveFlag(0b1110);
    pub const PROMO_CAP_QUEEN: MoveFlag = MoveFlag(0b1111);

    #[inline(always)]
    pub fn is_capture(self) -> bool {
        (self.0 & Self::CAPTURE_BIT) != 0
    }

    #[inline(always)]
    pub fn is_promo(self) -> bool {
        (self.0 & Self::PROMO_BIT) != 0
    }

    #[inline(always)]
    pub fn is_castle(self) -> bool {
        self.0 & 0b1110 == 0b0010
    }
}

use crate::r#const::SQ_TO_COORD;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move(pub u16);
// 0000   000000   000000 -> 16 bits
// flag     to      from  -> encoding format

impl Move {
    pub const NULL: Move = Move(0);

    pub fn new(from: usize, to: usize, flag: MoveFlag) -> Self {
        let m = (from as u16) | ((to as u16) << 6) | ((flag.0 as u16) << 12);
        Move(m)
    }

    #[inline(always)]
    pub fn from(self) -> usize {
        (self.0 & 0x3F) as usize
    }

    #[inline(always)]
    pub fn to(self) -> usize {
        ((self.0 >> 6) & 0x3F) as usize
    }

    #[inline(always)]
    pub fn flag(self) -> MoveFlag {
        MoveFlag(((self.0 >> 12) & 0xF) as FlagInfo)
    }

    pub fn to_coord(&self) -> String {
        let from = self.from();
        let to = self.to();
        let flag = self.flag();

        let mut coord = format!("{}{}", SQ_TO_COORD[from], SQ_TO_COORD[to]);

        if flag.is_promo() {
            // Mask the last 2 bits to get 0:N, 1:B, 2:R, 3:Q
            let promo_char = match flag.0 & MoveFlag::PIECE_BIT {
                MoveFlag::KNIGHT => 'n',
                MoveFlag::BISHOP => 'b',
                MoveFlag::ROOK => 'r',
                MoveFlag::QUEEN => 'q',
                _ => unreachable!(),
            };
            coord.push(promo_char);
        }

        coord
    }

    pub fn from_uci(mv_str: &str, board: &mut Board) -> Move {
        let move_list = board.gen_moves();

        for mv in move_list.moves {
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
    pub prev_last_irreversible: usize,
}

impl Undo {
    pub fn new(
        captured: PieceInfo,
        castling: CastlingRights,
        ensq: Option<u8>,
        last_irreversible: usize,
    ) -> Self {
        Self {
            captured,
            prev_en_passant_sq: ensq,
            prev_castling_rights: castling,
            prev_last_irreversible: last_irreversible,
        }
    }
}

const MAX_MOVES: usize = 256;
pub struct MoveList {
    pub moves: [Move; MAX_MOVES],
    pub len: usize,
}

impl MoveList {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            moves: [Move::NULL; MAX_MOVES],
            len: 0,
        }
    }

    #[inline(always)]
    pub fn get(&self, idx: usize) -> Move {
        self.moves[idx]
    }

    #[inline(always)]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.len < MAX_MOVES);
        self.moves[self.len] = mv;
        self.len += 1;
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
}

pub struct History {
    stack: [u64; 1024],
    len: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            stack: [0; 1024],
            len: 0,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn get(&self, idx: usize) -> u64 {
        self.stack[idx]
    }

    #[inline(always)]
    pub fn push(&mut self, key: u64) {
        debug_assert!(self.len < 1024);
        self.stack[self.len] = key;
        self.len += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> u64 {
        debug_assert!(self.len > 0);
        self.len -= 1;
        self.stack[self.len]
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    #[inline(always)]
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

    pub fn none() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub fn white_kingside(self) -> bool {
        self.0 & WK != 0
    }

    #[inline(always)]
    pub fn white_queenside(self) -> bool {
        self.0 & WQ != 0
    }

    #[inline(always)]
    pub fn black_kingside(self) -> bool {
        self.0 & BK != 0
    }

    #[inline(always)]
    pub fn black_queenside(self) -> bool {
        self.0 & BQ != 0
    }

    // remove rights
    #[inline(always)]
    pub fn remove(&mut self, mask: u8) {
        self.0 &= !mask;
    }

    // add rights
    #[inline(always)]
    pub fn add(&mut self, mask: u8) {
        self.0 |= mask;
    }
}
