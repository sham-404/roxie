use crate::items::*;
use crate::square::Square;

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

#[rustfmt::skip]
pub const WHITE_PAWN_ATTACKS: [u64; 64] = [
    0x200,              0x500,              0xa00,              0x1400,
    0x2800,             0x5000,             0xa000,             0x4000,
    0x20000,            0x50000,            0xa0000,            0x140000,
    0x280000,           0x500000,           0xa00000,           0x400000,
    0x2000000,          0x5000000,          0xa000000,          0x14000000,
    0x28000000,         0x50000000,         0xa0000000,         0x40000000,
    0x200000000,        0x500000000,        0xa00000000,        0x1400000000,
    0x2800000000,       0x5000000000,       0xa000000000,       0x4000000000,
    0x20000000000,      0x50000000000,      0xa0000000000,      0x140000000000,
    0x280000000000,     0x500000000000,     0xa00000000000,     0x400000000000,
    0x2000000000000,    0x5000000000000,    0xa000000000000,    0x14000000000000,
    0x28000000000000,   0x50000000000000,   0xa0000000000000,   0x40000000000000,
    0x200000000000000,  0x500000000000000,  0xa00000000000000,  0x1400000000000000,
    0x2800000000000000, 0x5000000000000000, 0xa000000000000000, 0x4000000000000000,
    0x0,                0x0,                0x0,                0x0,
    0x0,                0x0,                0x0,                0x0,
];

#[rustfmt::skip]
pub const BLACK_PAWN_ATTACKS: [u64; 64] = [
    0x0,              0x0,              0x0,              0x0,
    0x0,              0x0,              0x0,              0x0,
    0x2,              0x5,              0xa,              0x14,
    0x28,             0x50,             0xa0,             0x40,
    0x200,            0x500,            0xa00,            0x1400,
    0x2800,           0x5000,           0xa000,           0x4000,
    0x20000,          0x50000,          0xa0000,          0x140000,
    0x280000,         0x500000,         0xa00000,         0x400000,
    0x2000000,        0x5000000,        0xa000000,        0x14000000,
    0x28000000,       0x50000000,       0xa0000000,       0x40000000,
    0x200000000,      0x500000000,      0xa00000000,      0x1400000000,
    0x2800000000,     0x5000000000,     0xa000000000,     0x4000000000,
    0x20000000000,    0x50000000000,    0xa0000000000,    0x140000000000,
    0x280000000000,   0x500000000000,   0xa00000000000,   0x400000000000,
    0x2000000000000,  0x5000000000000,  0xa000000000000,  0x14000000000000,
    0x28000000000000, 0x50000000000000, 0xa0000000000000, 0x40000000000000,
];

pub const RANK2: u64 = 0x000000000000FF00;
pub const RANK7: u64 = 0x00FF000000000000;
pub const RANK8: u64 = 0xFF00000000000000;
pub const RANK1: u64 = 0x00000000000000FF;

#[inline]
pub fn pop_lsb(bb: &mut u64) -> Option<usize> {
    if *bb == 0 {
        return None;
    }

    let sq = bb.trailing_zeros() as usize;
    *bb &= *bb - 1;

    Some(sq)
}

pub struct Board {
    bitboards: [u64; 12],
    occupancy: [u64; 3],
    pub side_to_move: Color,
}

const WHITE: usize = 0;
const BLACK: usize = 1;
const BOTH: usize = 2;

impl Board {
    pub fn new() -> Self {
        let mut bitboards = [0u64; 12];

        // Black pieces
        bitboards[Piece::BP as usize] = 0x00FF000000000000;
        bitboards[Piece::BN as usize] = 0x4200000000000000;
        bitboards[Piece::BB as usize] = 0x2400000000000000;
        bitboards[Piece::BR as usize] = 0x8100000000000000;
        bitboards[Piece::BQ as usize] = 0x0800000000000000;
        bitboards[Piece::BK as usize] = 0x1000000000000000;

        // White pieces
        bitboards[Piece::WP as usize] = 0xFF00;
        bitboards[Piece::WN as usize] = 0x0042;
        bitboards[Piece::WB as usize] = 0x0024;
        bitboards[Piece::WR as usize] = 0x0081;
        bitboards[Piece::WQ as usize] = 0x0008;
        bitboards[Piece::WK as usize] = 0x0010;

        let occupancy: [u64; 3] = [0; 3];

        let mut board = Self {
            bitboards,
            occupancy,
            side_to_move: Color::White,
        };

        board.build_occupancy();
        board
    }

    fn build_occupancy(&mut self) {
        self.occupancy[WHITE] = self.bb(Piece::WP)
            | self.bb(Piece::WN)
            | self.bb(Piece::WB)
            | self.bb(Piece::WR)
            | self.bb(Piece::WQ)
            | self.bb(Piece::WK);

        self.occupancy[BLACK] = self.bb(Piece::BP)
            | self.bb(Piece::BN)
            | self.bb(Piece::BB)
            | self.bb(Piece::BR)
            | self.bb(Piece::BQ)
            | self.bb(Piece::BK);

        self.occupancy[BOTH] = self.occupancy[WHITE] | self.occupancy[BLACK];
    }

    pub fn move_piece(&mut self, mov: &Move) {
        let (from, to) = (Square::new(mov.from), Square::new(mov.to));

        let from_mask = 1u64 << from.index();
        let to_mask = 1u64 << to.index();

        // No piece in from
        if self.occupancy[BOTH] & from_mask == 0 {
            return;
        }

        // Handling captures
        for p in 0..12 {
            if to_mask & self.bitboards[p] != 0 {
                self.bitboards[p] ^= to_mask;
                break;
            }
        }

        for n in 0..12 {
            let piece = &mut self.bitboards[n];
            if from_mask & *piece != 0 {
                *piece ^= from_mask;
                *piece ^= to_mask;

                break;
            }
        }

        // Handling Special Moves
        match mov.flag {
            MoveFlag::PromoKnight | MoveFlag::PromoCapKnight => {
                let (pawn_bb, promo_bb) = match self.side_to_move {
                    Color::White => self.bb_mut_pair(Piece::WP, Piece::WN),
                    Color::Black => self.bb_mut_pair(Piece::BP, Piece::BN),
                };

                *pawn_bb ^= to_mask;
                *promo_bb ^= to_mask;
            }

            MoveFlag::PromoBishop | MoveFlag::PromoCapBishop => {
                let (pawn_bb, promo_bb) = match self.side_to_move {
                    Color::White => self.bb_mut_pair(Piece::WP, Piece::WB),
                    Color::Black => self.bb_mut_pair(Piece::BP, Piece::BB),
                };

                *pawn_bb ^= to_mask;
                *promo_bb ^= to_mask;
            }
            MoveFlag::PromoRook | MoveFlag::PromoCapRook => {
                let (pawn_bb, promo_bb) = match self.side_to_move {
                    Color::White => self.bb_mut_pair(Piece::WP, Piece::WR),
                    Color::Black => self.bb_mut_pair(Piece::BP, Piece::BR),
                };

                *pawn_bb ^= to_mask;
                *promo_bb ^= to_mask;
            }
            MoveFlag::PromoQueen | MoveFlag::PromoCapQueen => {
                let (pawn_bb, promo_bb) = match self.side_to_move {
                    Color::White => self.bb_mut_pair(Piece::WP, Piece::WQ),
                    Color::Black => self.bb_mut_pair(Piece::BP, Piece::BQ),
                };

                *pawn_bb ^= to_mask;
                *promo_bb ^= to_mask;
            }
            _ => {}
        }
        self.build_occupancy();
    }

    pub fn occ(&self, color: &Color) -> u64 {
        match color {
            Color::White => self.occupancy[WHITE],
            Color::Black => self.occupancy[BLACK],
        }
    }

    pub fn all_occ(&self) -> u64 {
        self.occupancy[BOTH]
    }

    pub fn bb(&self, piece: Piece) -> u64 {
        self.bitboards[piece as usize]
    }

    fn bb_mut_pair(&mut self, a: Piece, b: Piece) -> (&mut u64, &mut u64) {
        let ai = a as usize;
        let bi = b as usize;

        assert!(ai != bi);

        if ai < bi {
            let (left, right) = self.bitboards.split_at_mut(bi);
            (&mut left[ai], &mut right[0])
        } else {
            let (left, right) = self.bitboards.split_at_mut(ai);
            (&mut right[0], &mut left[bi])
        }
    }
}

////////// Move generations
impl Board {
    pub fn gen_moves(&self) -> Vec<Move> {
        let mut moves: Vec<Move> = Vec::new();

        self.gen_king_moves(&mut moves);
        self.gen_knight_moves(&mut moves);
        self.gen_pawn_moves(&mut moves);

        let (bishop_bb, rook_bb, queen_bb) = match self.side_to_move {
            Color::White => (self.bb(Piece::WB), self.bb(Piece::WR), self.bb(Piece::WQ)),
            Color::Black => (self.bb(Piece::BB), self.bb(Piece::BR), self.bb(Piece::BQ)),
        };

        const ROOK_DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        const BISHOP_DIRS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        const QUEEN_DIRS: [(i32, i32); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        self.gen_sliding_moves(&mut moves, bishop_bb, &BISHOP_DIRS);
        self.gen_sliding_moves(&mut moves, rook_bb, &ROOK_DIRS);
        self.gen_sliding_moves(&mut moves, queen_bb, &QUEEN_DIRS);

        moves
    }

    pub fn gen_king_moves(&self, moves: &mut Vec<Move>) {
        let piece = match self.side_to_move {
            Color::White => Piece::WK,
            Color::Black => Piece::BK,
        };

        let mut bb = self.bb(piece);

        while let Some(from) = pop_lsb(&mut bb) {
            let to_move = &self.side_to_move;
            let occ = self.occ(to_move);

            let mut atk = KING_ATTACKS[from] & !occ;

            while let Some(to) = pop_lsb(&mut atk) {
                let flag = if (1 << to) & self.occ(&self.side_to_move.opponent()) != 0 {
                    MoveFlag::Capture
                } else {
                    MoveFlag::Quiet
                };

                moves.push(Move::new(from, to, flag));
            }
        }
    }

    pub fn gen_knight_moves(&self, moves: &mut Vec<Move>) {
        let piece = match self.side_to_move {
            Color::White => Piece::WN,
            Color::Black => Piece::BN,
        };

        let mut bb = self.bb(piece);

        while let Some(from) = pop_lsb(&mut bb) {
            let to_move = &self.side_to_move;
            let occ = self.occ(to_move);

            let mut atk = KNIGHT_ATTACKS[from] & !occ;

            while let Some(to) = pop_lsb(&mut atk) {
                let flag = if (1 << to) & self.occ(&self.side_to_move.opponent()) != 0 {
                    MoveFlag::Capture
                } else {
                    MoveFlag::Quiet
                };

                moves.push(Move::new(from, to, flag));
            }
        }
    }

    pub fn gen_pawn_moves(&self, moves: &mut Vec<Move>) {
        let (piece, start_pos, end_pos, attacks, dir) = match self.side_to_move {
            Color::White => (Piece::WP, RANK2, RANK8, WHITE_PAWN_ATTACKS, 8i8),
            Color::Black => (Piece::BP, RANK7, RANK1, BLACK_PAWN_ATTACKS, -8i8),
        };

        let pawn_bb = self.bb(piece);
        let empty = !self.all_occ();

        // forward pawn moves
        let (mut single, mut double) = match piece {
            Piece::WP => {
                let single = (pawn_bb << 8) & empty;
                let double = ((pawn_bb & start_pos) << 16) & empty & (empty << 8);
                (single, double)
            }
            Piece::BP => {
                let single = (pawn_bb >> 8) & empty;
                let double = ((pawn_bb & start_pos) >> 16) & empty & (empty >> 8);
                (single, double)
            }
            _ => unreachable!(),
        };

        // Single push
        while let Some(to) = pop_lsb(&mut single) {
            let from = (to as i8 - dir) as usize;

            // Handling Quiet Promotions
            if (1u64 << to) & end_pos != 0 {
                moves.push(Move::new(from, to, MoveFlag::PromoKnight));
                moves.push(Move::new(from, to, MoveFlag::PromoRook));
                moves.push(Move::new(from, to, MoveFlag::PromoQueen));
                moves.push(Move::new(from, to, MoveFlag::PromoBishop));
            } else {
                moves.push(Move::new(from, to, MoveFlag::Quiet));
            }
        }

        // Double push
        while let Some(to) = pop_lsb(&mut double) {
            let from = (to as i8 - (2 * dir)) as usize;
            moves.push(Move::new(from, to, MoveFlag::Quiet));
        }

        // Captures
        let mut bb = pawn_bb;
        let enemy = self.occ(&self.side_to_move.opponent());

        while let Some(from) = pop_lsb(&mut bb) {
            let mut atk = attacks[from] & enemy;

            while let Some(to) = pop_lsb(&mut atk) {
                // Handling Capture Promotions
                if (1u64 << to) & end_pos != 0 {
                    moves.push(Move::new(from, to, MoveFlag::PromoCapKnight));
                    moves.push(Move::new(from, to, MoveFlag::PromoCapRook));
                    moves.push(Move::new(from, to, MoveFlag::PromoCapQueen));
                    moves.push(Move::new(from, to, MoveFlag::PromoCapBishop));
                } else {
                    moves.push(Move::new(from, to, MoveFlag::Capture));
                }
            }
        }
    }

    pub fn gen_sliding_moves(&self, moves: &mut Vec<Move>, mut bb: u64, directions: &[(i32, i32)]) {
        let own_occ = self.occ(&self.side_to_move);
        let enemy_occ = self.occ(&self.side_to_move.opponent());

        while let Some(from_idx) = pop_lsb(&mut bb) {
            let from = Square::new(from_idx);

            for &(dr, df) in directions {
                let mut sq = from;

                while let Some(next) = sq.offset(dr, df) {
                    let to_bb = 1u64 << next.index();

                    // blocked by own piece
                    if to_bb & own_occ != 0 {
                        break;
                    }

                    let flag = if to_bb & enemy_occ != 0 {
                        MoveFlag::Capture
                    } else {
                        MoveFlag::Quiet
                    };

                    moves.push(Move::new(from_idx, next.index(), flag));

                    // stop after capture
                    if to_bb & enemy_occ != 0 {
                        break;
                    }

                    sq = next;
                }
            }
        }
    }
}

////////// Debugging board
#[allow(dead_code)]
impl Board {
    pub fn render_board(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Top border
        let mut top = String::from("   ");
        for file in 0..8 {
            if file == 0 {
                top.push_str("┌───");
            } else {
                top.push_str("┬───");
            }
        }
        top.push('┐');
        lines.push(top);

        for rank in (0..8).rev() {
            let mut row = format!("{}  ", rank + 1);

            for file in 0..8 {
                let sq = rank * 8 + file;
                let mut found = false;

                for i in 0..12 {
                    if (self.bitboards[i] >> sq) & 1 == 1 {
                        let piece = Piece::from_val(i);
                        row.push_str(&format!("│ {} ", Piece::piece_to_char(piece)));
                        found = true;
                        break;
                    }
                }

                if !found {
                    row.push_str("│   ");
                }
            }

            row.push('│');
            lines.push(row);

            if rank > 0 {
                let mut sep = String::from("   ");
                for file in 0..8 {
                    if file == 0 {
                        sep.push_str("├───");
                    } else {
                        sep.push_str("┼───");
                    }
                }
                sep.push('┤');
                lines.push(sep);
            }
        }

        // Bottom border
        let mut bottom = String::from("   ");
        for file in 0..8 {
            if file == 0 {
                bottom.push_str("└───");
            } else {
                bottom.push_str("┴───");
            }
        }
        bottom.push('┘');
        lines.push(bottom);

        lines.push("     a   b   c   d   e   f   g   h".to_string());

        lines
    }

    pub fn print_many(&self, boards: Vec<Vec<String>>) {
        let height = boards[0].len();

        for i in 0..height {
            for board in &boards {
                print!("{:<40} ", board[i]);
            }
            println!();
        }
    }

    pub fn render_bitboard(&self, bb: u64) -> Vec<String> {
        let mut lines = Vec::new();

        // Top border
        let mut top = String::from("   ");
        for file in 0..8 {
            if file == 0 {
                top.push_str("┌───");
            } else {
                top.push_str("┬───");
            }
        }
        top.push('┐');
        lines.push(top);

        for rank in (0..8).rev() {
            let mut row = format!("{}  ", rank + 1);

            for file in 0..8 {
                let sq = rank * 8 + file;
                if (bb >> sq) & 1 == 1 {
                    row.push_str("│ X ");
                } else {
                    row.push_str("│ . ");
                }
            }

            row.push('│');
            lines.push(row);

            if rank > 0 {
                let mut sep = String::from("   ");
                for file in 0..8 {
                    if file == 0 {
                        sep.push_str("├───");
                    } else {
                        sep.push_str("┼───");
                    }
                }
                sep.push('┤');
                lines.push(sep);
            }
        }

        // Bottom border
        let mut bottom = String::from("   ");
        for file in 0..8 {
            if file == 0 {
                bottom.push_str("└───");
            } else {
                bottom.push_str("┴───");
            }
        }
        bottom.push('┘');
        lines.push(bottom);

        lines.push("     a   b   c   d   e   f   g   h".to_string());

        lines
    }
}
