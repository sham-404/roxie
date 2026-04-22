use crate::board::Board;
use crate::board::Color;
use crate::board::Move;
use crate::board::MoveFlag;
use crate::board::Piece;

#[rustfmt::skip]
const KING_ATTACKS: [u64; 64] = [
    0x302,              0x705,              0xe0a,              0x1c14,
    0x3828,             0x7050,             0xe0a0,             0xc040,
    0x30203,            0x70507,            0xe0a0e,            0x1c141c,
    0x382838,           0x705070,           0xe0a0e0,           0xc040c0,
    0x3020300,          0x7050700,          0xe0a0e00,          0x1c141c00,
    0x38283800,         0x70507000,         0xe0a0e000,         0xc040c000,
    0x302030000,        0x705070000,        0xe0a0e0000,        0x1c141c0000,
    0x3828380000,       0x7050700000,       0xe0a0e00000,       0xc040c00000,
    0x30203000000,      0x70507000000,      0xe0a0e000000,      0x1c141c000000,
    0x382838000000,     0x705070000000,     0xe0a0e0000000,     0xc040c0000000,
    0x3020300000000,    0x7050700000000,    0xe0a0e00000000,    0x1c141c00000000,
    0x38283800000000,   0x70507000000000,   0xe0a0e000000000,   0xc040c000000000,
    0x302030000000000,  0x705070000000000,  0xe0a0e0000000000,  0x1c141c0000000000,
    0x3828380000000000, 0x7050700000000000, 0xe0a0e00000000000, 0xc040c00000000000,
    0x203000000000000,  0x507000000000000,  0xa0e000000000000,  0x141c000000000000,
    0x2838000000000000, 0x5070000000000000, 0xa0e0000000000000, 0x40c0000000000000,
];

#[rustfmt::skip]
pub const KNIGHT_ATTACKS: [u64; 64] = [
    0x20400,            0x50800,            0xa1100,            0x142200,
    0x284400,           0x508800,           0xa01000,           0x402000,
    0x2040004,          0x5080008,          0xa110011,          0x14220022,
    0x28440044,         0x50880088,         0xa0100010,         0x40200020,
    0x204000402,        0x508000805,        0xa1100110a,        0x1422002214,
    0x2844004428,       0x5088008850,       0xa0100010a0,       0x4020002040,
    0x20400040200,      0x50800080500,      0xa1100110a00,      0x142200221400,
    0x284400442800,     0x508800885000,     0xa0100010a000,     0x402000204000,
    0x2040004020000,    0x5080008050000,    0xa1100110a0000,    0x14220022140000,
    0x28440044280000,   0x50880088500000,   0xa0100010a00000,   0x40200020400000,
    0x204000402000000,  0x508000805000000,  0xa1100110a000000,  0x1422002214000000,
    0x2844004428000000, 0x5088008850000000, 0xa0100010a0000000, 0x4020002040000000,
    0x400040200000000,  0x800080500000000,  0x1100110a00000000, 0x2200221400000000,
    0x4400442800000000, 0x8800885000000000, 0x100010a000000000, 0x2000204000000000,
    0x4020000000000,    0x8050000000000,    0x110a0000000000,   0x22140000000000,
    0x44280000000000,   0x0088500000000000, 0x0010a00000000000, 0x20400000000000,
];

impl Board {
    #[inline]
    pub fn pop_lsb(&self, bb: &mut u64) -> Option<usize> {
        if *bb == 0 {
            return None;
        }

        let sq = bb.trailing_zeros() as usize;
        *bb &= *bb - 1;

        Some(sq)
    }

    pub fn gen_king_attack(&mut self) -> Vec<Move> {
        let mut moves: Vec<Move> = Vec::new();

        let piece = match self.side_to_move {
            Color::White => Piece::WK,
            Color::Black => Piece::BK,
        };

        let mut bb = self.bb(piece);

        while let Some(from) = self.pop_lsb(&mut bb) {
            let to_move = &self.side_to_move;
            let occ = self.occ(to_move);

            let mut atk = KING_ATTACKS[from] & !occ;

            while let Some(to) = self.pop_lsb(&mut atk) {
                let flag = if (1 << to) & self.occ(&self.side_to_move.opponent()) != 0 {
                    MoveFlag::Capture
                } else {
                    MoveFlag::Quiet
                };

                moves.push(Move::new(from, to, flag));
            }
        }

        moves
    }

    pub fn gen_knight_attack(&mut self) -> Vec<Move> {
        let mut moves: Vec<Move> = Vec::new();

        let piece = match self.side_to_move {
            Color::White => Piece::WN,
            Color::Black => Piece::BN,
        };

        let mut bb = self.bb(piece);

        while let Some(from) = self.pop_lsb(&mut bb) {
            let to_move = &self.side_to_move;
            let occ = self.occ(to_move);

            let mut atk = KNIGHT_ATTACKS[from] & !occ;

            while let Some(to) = self.pop_lsb(&mut atk) {
                let flag = if (1 << to) & self.occ(&self.side_to_move.opponent()) != 0 {
                    MoveFlag::Capture
                } else {
                    MoveFlag::Quiet
                };

                moves.push(Move::new(from, to, flag));
            }
        }

        moves
    }
}
