use crate::r#const::*;
use crate::items::*;
use crate::square::Square;

#[inline]
pub fn pop_lsb(bb: &mut u64) -> Option<usize> {
    if *bb == 0 {
        return None;
    }

    let sq = bb.trailing_zeros() as usize;
    *bb &= *bb - 1;

    Some(sq)
}

#[inline]
pub fn mask(idx: usize) -> u64 {
    #[rustfmt::skip]
    const SQUARE_BB: [u64; 65] = [
        0x1,                0x2,                0x4,                0x8,
        0x10,               0x20,               0x40,               0x80,
        0x100,              0x200,              0x400,              0x800,
        0x1000,             0x2000,             0x4000,             0x8000,
        0x10000,            0x20000,            0x40000,            0x80000,
        0x100000,           0x200000,           0x400000,           0x800000,
        0x1000000,          0x2000000,          0x4000000,          0x8000000,
        0x10000000,         0x20000000,         0x40000000,         0x80000000,
        0x100000000,        0x200000000,        0x400000000,        0x800000000,
        0x1000000000,       0x2000000000,       0x4000000000,       0x8000000000,
        0x10000000000,      0x20000000000,      0x40000000000,      0x80000000000,
        0x100000000000,     0x200000000000,     0x400000000000,     0x800000000000,
        0x1000000000000,    0x2000000000000,    0x4000000000000,    0x8000000000000,
        0x10000000000000,   0x20000000000000,   0x40000000000000,   0x80000000000000,
        0x100000000000000,  0x200000000000000,  0x400000000000000,  0x800000000000000,
        0x1000000000000000, 0x2000000000000000, 0x4000000000000000, 0x8000000000000000,
        0x0,
    ];

    SQUARE_BB[idx]
}

#[derive(Clone)]
pub struct Board {
    bitboards: [u64; 12],
    occupancy: [u64; 3],
    castling: CastlingRights,
    side_to_move: Color,
    en_passant: Option<u8>,
}

impl Board {
    pub fn new() -> Self {
        let bitboards = [0u64; 12];

        let occupancy: [u64; 3] = [0; 3];

        let board = Self {
            bitboards,
            occupancy,
            side_to_move: Color::White,
            castling: CastlingRights::new(),
            en_passant: None,
        };

        board
    }

    pub fn start_pos() -> Self {
        let mut bitboards = [0u64; 12];

        let occupancy: [u64; 3] = [0; 3];

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

        let mut board = Self {
            bitboards,
            occupancy,
            side_to_move: Color::White,
            castling: CastlingRights::new(),
            en_passant: None,
        };

        board.build_occupancy();

        board
    }

    pub fn load_fen(fen: &str) -> Self {
        let mut board = Self::new();

        board.bitboards = [0; 12];
        board.occupancy = [0; 3];
        board.en_passant = None;
        board.castling = CastlingRights::new();

        let mut parts = fen.split_whitespace();

        let piece_part = parts.next().expect("Invalid FEN");
        let side_part = parts.next().expect("Invalid FEN");
        let castling_part = parts.next().expect("Invalid FEN");
        let ep_part = parts.next().expect("Invalid FEN");

        let mut rank: i32 = 7;
        let mut file: i32 = 0;

        for c in piece_part.chars() {
            match c {
                '/' => {
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => {
                    file += c.to_digit(10).unwrap() as i32;
                }
                _ => {
                    if let Some(piece) = Piece::from_char(c) {
                        let sq = (rank * 8 + file) as usize;
                        *board.mut_bb(piece) |= mask(sq);
                        file += 1;
                    } else {
                        panic!("Invalid piece char in FEN: {}", c);
                    }
                }
            }
        }

        board.side_to_move = match side_part {
            "w" => Color::White,
            "b" => Color::Black,
            _ => panic!("Invalid side to move"),
        };

        if castling_part != "-" {
            for c in castling_part.chars() {
                match c {
                    'K' => board.castling.add(WK),
                    'Q' => board.castling.add(WQ),
                    'k' => board.castling.add(BK),
                    'q' => board.castling.add(BQ),
                    _ => panic!("Invalid castling char"),
                }
            }
        }

        if ep_part != "-" {
            let bytes = ep_part.as_bytes();
            let file = (bytes[0] - b'a') as usize;
            let rank = (bytes[1] - b'1') as usize;
            let sq = rank * 8 + file;
            board.en_passant = Some(sq as u8);
        }

        board.build_occupancy();

        board
    }
    pub fn make_move(&mut self, mov: &Move) -> Undo {
        debug_assert!(self.occupancy[BOTH] & mask(mov.from) != 0);

        let cur_piece = self
            .piece_on(mov.from)
            .expect("make_move(): Why there is no piece of mov.from?");

        // Detecting captures
        let (captured, captured_sq) = match mov.flag {
            MoveFlag::EnPassant => {
                let captured_sq = match self.side_to_move {
                    Color::White => mov.to - 8,
                    Color::Black => mov.to + 8,
                };

                (self.piece_on(captured_sq), captured_sq)
            }
            _ => (self.piece_on(mov.to), mov.to),
        };

        // Constructing undo
        let undo = Undo::new(captured, self.castling, self.en_passant);

        // Handling captures
        if let Some(cap_piece) = captured {
            self.remove_piece(cap_piece, captured_sq);
        }

        // Moving the piece on the board
        self.move_piece_quiet(mov.from, mov.to);

        // Handling Special Moves (for piece state)
        // Promotions
        let promo_piece = match self.side_to_move {
            Color::White => match mov.flag {
                MoveFlag::PromoKnight | MoveFlag::PromoCapKnight => Some(Piece::WN),
                MoveFlag::PromoBishop | MoveFlag::PromoCapBishop => Some(Piece::WB),
                MoveFlag::PromoRook | MoveFlag::PromoCapRook => Some(Piece::WR),
                MoveFlag::PromoQueen | MoveFlag::PromoCapQueen => Some(Piece::WQ),
                _ => None,
            },
            Color::Black => match mov.flag {
                MoveFlag::PromoKnight | MoveFlag::PromoCapKnight => Some(Piece::BN),
                MoveFlag::PromoBishop | MoveFlag::PromoCapBishop => Some(Piece::BB),
                MoveFlag::PromoRook | MoveFlag::PromoCapRook => Some(Piece::BR),
                MoveFlag::PromoQueen | MoveFlag::PromoCapQueen => Some(Piece::BQ),
                _ => None,
            },
        };

        if let Some(promo) = promo_piece {
            // Removing the pawn, which is in the promotion square
            self.remove_piece(cur_piece, mov.to);

            // Adding the promotion piece
            self.add_piece(promo, mov.to);
        }

        // castling
        match mov.flag {
            MoveFlag::KingCastle => {
                let king_pos = match cur_piece {
                    Piece::WK => WK_START_POS,
                    Piece::BK => BK_START_POS,
                    _ => unreachable!("Non king move got Castle flag"),
                };

                self.move_piece_quiet(king_pos + 3, king_pos + 1);
            }
            MoveFlag::QueenCastle => {
                let king_pos = match cur_piece {
                    Piece::WK => WK_START_POS,
                    Piece::BK => BK_START_POS,
                    _ => unreachable!("Non king move got Castle flag"),
                };

                self.move_piece_quiet(king_pos - 4, king_pos - 1);
            }
            _ => {}
        }

        /////// Handling Special moves (for board state)
        // updating en_passant square
        self.en_passant = match mov.flag {
            MoveFlag::DoublePush => Some(((mov.from + mov.to) / 2) as u8),
            _ => None,
        };

        //// Handling castling rights
        // White king moves
        if mov.from == WK_START_POS {
            self.castling.remove(WK | WQ);
        }

        // Black king moves
        if mov.from == BK_START_POS {
            self.castling.remove(BK | BQ);
        }

        // If white kingside rook moved, or captured
        if mov.from == WK_START_POS + 3 || mov.to == WK_START_POS + 3 {
            self.castling.remove(WK);
        }

        // If white queenside rook moved, or captured
        if mov.from == WK_START_POS - 4 || mov.to == WK_START_POS - 4 {
            self.castling.remove(WQ);
        }

        // If black kingside rook moved, or captured
        if mov.from == BK_START_POS + 3 || mov.to == BK_START_POS + 3 {
            self.castling.remove(BK);
        }

        // If black queenside rook moved, or captured
        if mov.from == BK_START_POS - 4 || mov.to == BK_START_POS - 4 {
            self.castling.remove(BQ);
        }

        // Post move activities
        self.build_occupancy();

        self.side_to_move = self.side_to_move.opponent();

        undo
    }

    pub fn undo_move(&mut self, mov: &Move, undo: &Undo) {
        let cur_piece = self
            .piece_on(mov.to)
            .expect("undo_move(): piece is not on mov.to");

        // move piece back
        self.move_piece_quiet(mov.to, mov.from);

        // handle captures
        match mov.flag {
            MoveFlag::Capture
            | MoveFlag::PromoCapQueen
            | MoveFlag::PromoCapRook
            | MoveFlag::PromoCapBishop
            | MoveFlag::PromoCapKnight => {
                let cap_piece = undo.captured.expect("Capture without captured piece");
                self.add_piece(cap_piece, mov.to);
            }
            MoveFlag::EnPassant => {
                let cap_piece = undo.captured.expect("EP without captured piece");

                let cap_sq = if cap_piece == Piece::BP {
                    mov.to - 8
                } else {
                    mov.to + 8
                };

                self.add_piece(cap_piece, cap_sq);
            }
            _ => {}
        }

        // Handling Promotions
        match mov.flag {
            MoveFlag::PromoQueen
            | MoveFlag::PromoRook
            | MoveFlag::PromoBishop
            | MoveFlag::PromoKnight
            | MoveFlag::PromoCapQueen
            | MoveFlag::PromoCapRook
            | MoveFlag::PromoCapBishop
            | MoveFlag::PromoCapKnight => {
                // restore pawn
                let pawn = match cur_piece {
                    Piece::WQ | Piece::WR | Piece::WB | Piece::WN => Piece::WP,
                    Piece::BQ | Piece::BR | Piece::BB | Piece::BN => Piece::BP,
                    _ => unreachable!("promotion undo with non-promoted piece"),
                };

                // Removing the Promoted piece from mov.from
                // (cuz it got added when we try to undo the move
                self.remove_piece(cur_piece, mov.from);
                // adding the relevent pawn on Promotion moves
                self.add_piece(pawn, mov.from);
            }
            _ => {}
        }

        // castling
        match mov.flag {
            MoveFlag::KingCastle => {
                // rook: f -> h
                self.move_piece_quiet(mov.to - 1, mov.to + 1);
            }
            MoveFlag::QueenCastle => {
                // rook: d -> a
                self.move_piece_quiet(mov.to + 1, mov.to - 2);
            }
            _ => {}
        }

        // restore state
        self.en_passant = undo.prev_en_passant_sq;
        self.castling = undo.prev_castling_rights;

        self.build_occupancy();

        self.side_to_move = self.side_to_move.opponent();
    }

    fn is_square_atacked(&self, pos: usize, cur_color: &Color) -> bool {
        let all_occ = self.all_occ();

        let (en_pawn, en_king, en_queen, en_bishop, en_rook, en_knight) = match cur_color {
            Color::White => (
                self.bb(Piece::BP),
                self.bb(Piece::BK),
                self.bb(Piece::BQ),
                self.bb(Piece::BB),
                self.bb(Piece::BR),
                self.bb(Piece::BN),
            ),
            Color::Black => (
                self.bb(Piece::WP),
                self.bb(Piece::WK),
                self.bb(Piece::WQ),
                self.bb(Piece::WB),
                self.bb(Piece::WR),
                self.bb(Piece::WN),
            ),
        };

        // Sliding pieces
        let directions = [
            ([(1, 1), (1, -1), (-1, 1), (-1, -1)], true), // diagonals
            ([(1, 0), (-1, 0), (0, 1), (0, -1)], false),  // straight
        ];
        let from = Square::new(pos);

        for (dir, is_diag) in directions {
            for (dr, df) in dir {
                let mut sq = from;

                while let Some(next) = sq.offset(dr, df) {
                    let to_bb = mask(next.index());

                    // Some piece is blocking our way
                    if to_bb & all_occ != 0 {
                        // Enemy queen attacks
                        if en_queen & to_bb != 0 {
                            return true;
                        }

                        // Enemy bishop attacks
                        if (en_bishop & to_bb != 0) && is_diag {
                            return true;
                        }

                        // Enemy rook attacks
                        if (en_rook & to_bb != 0) && !is_diag {
                            return true;
                        }
                        // return true if the blocking piece is an enemy rook,
                        // bishop or a queen, else break the loop as we have
                        // been blocked by our own piece, or an non sliding
                        // enemy piece

                        break;
                    }
                    sq = next;
                }
            }
        }

        // Knights
        let possible_knight_atk_sq = KNIGHT_ATTACKS[pos];

        // Enemy knight is attacking
        if en_knight & possible_knight_atk_sq != 0 {
            return true;
        }

        // King
        let possible_king_atk_sq = KING_ATTACKS[pos];

        // Enemy King attacks
        if en_king & possible_king_atk_sq != 0 {
            return true;
        }

        // if a opp pawn is in cur color pawn's attacking sq, then
        // the opponent pawn is attacking the current sq
        let possible_pawn_atk_sq = match cur_color {
            Color::White => WHITE_PAWN_ATTACKS[pos],
            Color::Black => BLACK_PAWN_ATTACKS[pos],
        };

        if en_pawn & possible_pawn_atk_sq != 0 {
            return true;
        }

        false
    }

    #[inline]
    pub fn piece_on(&self, sq: usize) -> Option<Piece> {
        let mask = mask(sq);

        for p in 0..12 {
            if self.bitboards[p] & mask != 0 {
                return Some(Piece::from_val(p));
            }
        }

        None
    }
}

///////// Helpers
impl Board {
    fn move_piece_quiet(&mut self, from: usize, to: usize) {
        let (from_mask, to_mask) = (mask(from), mask(to));
        let piece = self.piece_on(from).expect("There ain't no piece in from");

        let piece_bb = self.mut_bb(piece);
        *piece_bb &= !from_mask;
        *piece_bb |= to_mask;
    }

    fn remove_piece(&mut self, piece: Piece, pos: usize) {
        let pos_mask = mask(pos);

        *self.mut_bb(piece) &= !pos_mask;
    }

    fn add_piece(&mut self, piece: Piece, pos: usize) {
        let pos_mask = mask(pos);

        *self.mut_bb(piece) |= pos_mask;
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

    pub fn mut_bb(&mut self, piece: Piece) -> &mut u64 {
        &mut self.bitboards[piece as usize]
    }
}

////////// Move generations
impl Board {
    pub fn gen_moves(&mut self) -> Vec<Move> {
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

        self.gen_castling_moves(&mut moves);

        self.filter_illegal(moves)
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
            if mask(to) & end_pos != 0 {
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
            moves.push(Move::new(from, to, MoveFlag::DoublePush));
        }

        // Captures
        let mut bb = pawn_bb;
        let enemy = self.occ(&self.side_to_move.opponent());

        while let Some(from) = pop_lsb(&mut bb) {
            // To include en_passant sq, as there wont be any enemy there
            // (must be handled explicitely while creating the Move)
            let target = match self.en_passant {
                Some(sq) => enemy | mask(sq as usize),
                None => enemy,
            };

            let mut atk = attacks[from] & target;

            while let Some(to) = pop_lsb(&mut atk) {
                // Handling en_passant
                if let Some(sq) = self.en_passant {
                    if sq as usize == to {
                        moves.push(Move::new(from, to, MoveFlag::EnPassant));
                        continue;
                    }
                }

                if mask(to) & end_pos != 0 {
                    // Handling Capture Promotions
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
                    let to_bb = mask(next.index());

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

    fn gen_castling_moves(&self, moves: &mut Vec<Move>) {
        let (king_piece, color, ks_rights, qs_rights) = match self.side_to_move {
            Color::White => (
                Piece::WK,
                Color::White,
                self.castling.white_kingside(),
                self.castling.white_queenside(),
            ),
            Color::Black => (
                Piece::BK,
                Color::Black,
                self.castling.black_kingside(),
                self.castling.black_queenside(),
            ),
        };

        let mut king_bb = self.bb(king_piece);
        let king_pos = pop_lsb(&mut king_bb).expect("There is no King!!!");

        if ks_rights && self.can_castle_kingside(king_pos, &color) {
            moves.push(Move::new(king_pos, king_pos + 2, MoveFlag::KingCastle));
        }

        if qs_rights && self.can_castle_queenside(king_pos, &color) {
            moves.push(Move::new(king_pos, king_pos - 2, MoveFlag::QueenCastle));
        }
    }

    fn filter_illegal(&mut self, moves: Vec<Move>) -> Vec<Move> {
        let mut legal: Vec<Move> = Vec::new();

        let king = match self.side_to_move {
            Color::White => Piece::WK,
            Color::Black => Piece::BK,
        };

        let color = self.side_to_move;

        for mv in &moves {
            let undo = self.make_move(&mv);

            let mut king_bb = self.bb(king);
            let king_pos = pop_lsb(&mut king_bb).expect("There is no King!!!");

            if !self.is_square_atacked(king_pos, &color) {
                legal.push(*mv);
            }

            self.undo_move(&mv, &undo);
        }

        legal
    }

    fn can_castle_kingside(&self, king_pos: usize, color: &Color) -> bool {
        let (start_pos, rook) = match color {
            Color::White => (WK_START_POS, Piece::WR),
            Color::Black => (BK_START_POS, Piece::BR),
        };

        let occ = self.all_occ();

        // return true if king is in start pos
        // king is not in attack
        // adjacent squares are Empty
        // rook is in place
        // adjacent Empty squares are not in attack

        start_pos == king_pos
            && !self.is_square_atacked(king_pos, &color)
            && occ & (mask(king_pos + 1) | mask(king_pos + 2)) == 0
            && self.piece_on(king_pos + 3) == Some(rook)
            && !self.is_square_atacked(king_pos + 1, &color)
            && !self.is_square_atacked(king_pos + 2, &color)
    }

    fn can_castle_queenside(&self, king_pos: usize, color: &Color) -> bool {
        let (start_pos, rook) = match color {
            Color::White => (WK_START_POS, Piece::WR),
            Color::Black => (BK_START_POS, Piece::BR),
        };

        let occ = self.all_occ();

        // return true if king is in start pos
        // king is not in attack
        // adjacent squares are Empty
        // rook is in place
        // adjacent Empty squares are not in attack

        start_pos == king_pos
            && !self.is_square_atacked(king_pos, &color)
            && occ & (mask(king_pos - 1) | mask(king_pos - 2) | mask(king_pos - 3)) == 0
            && self.piece_on(king_pos - 4) == Some(rook)
            && !self.is_square_atacked(king_pos - 1, &color)
            && !self.is_square_atacked(king_pos - 2, &color)
    }
}

////////// Debugging board (enhanced)

#[allow(dead_code)]
impl Board {
    pub fn render_board_debug(&self, cursor: u64, selected: u64, moves: u64) -> Vec<String> {
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

                let is_cursor = (cursor >> sq) & 1 == 1;
                let is_selected = (selected >> sq) & 1 == 1;
                let is_move = (moves >> sq) & 1 == 1;

                let mut found = false;

                for i in 0..12 {
                    if (self.bitboards[i] >> sq) & 1 == 1 {
                        let piece = Piece::from_val(i);
                        let glyph = Piece::piece_to_char(piece);

                        // Priority: cursor > selected > moves
                        if is_cursor {
                            row.push_str(&format!("│[{}]", glyph));
                        } else if is_selected {
                            row.push_str(&format!("│({})", glyph));
                        } else if is_move {
                            row.push_str(&format!("│ {}*", glyph));
                        } else {
                            row.push_str(&format!("│ {} ", glyph));
                        }

                        found = true;
                        break;
                    }
                }

                if !found {
                    if is_cursor {
                        row.push_str("│[ ]");
                    } else if is_selected {
                        row.push_str("│(*)");
                    } else if is_move {
                        row.push_str("│ * ");
                    } else {
                        row.push_str("│   ");
                    }
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
}
