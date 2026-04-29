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
    1u64 << idx
}

#[derive(Clone)]
pub struct Board {
    bitboards: [u64; 12],
    occupancy: [u64; 3],
    mailbox: [PieceInfo; 64],
    castling: CastlingRights,
    side_to_move: Color,
    en_passant: Option<u8>,
}

impl Board {
    pub fn new() -> Self {
        let bitboards = [0u64; 12];

        let occupancy: [u64; 3] = [0; 3];
        let mailbox: [PieceInfo; 64] = [0u8; 64];

        let board = Self {
            bitboards,
            occupancy,
            mailbox,
            side_to_move: Color::White,
            castling: CastlingRights::new(),
            en_passant: None,
        };

        board
    }

    pub fn start_pos() -> Self {
        let occupancy: [u64; 3] = [0; 3];

        let mut bitboards = [0u64; 12];

        // White pieces
        bitboards[Piece::to_idx(Piece::WHITE | Piece::PAWN)] = 0xFF00;
        bitboards[Piece::to_idx(Piece::WHITE | Piece::KNIGHT)] = 0x0042;
        bitboards[Piece::to_idx(Piece::WHITE | Piece::BISHOP)] = 0x0024;
        bitboards[Piece::to_idx(Piece::WHITE | Piece::ROOK)] = 0x0081;
        bitboards[Piece::to_idx(Piece::WHITE | Piece::QUEEN)] = 0x0008;
        bitboards[Piece::to_idx(Piece::WHITE | Piece::KING)] = 0x0010;

        // Black pieces
        bitboards[Piece::to_idx(Piece::BLACK | Piece::PAWN)] = 0x00FF000000000000;
        bitboards[Piece::to_idx(Piece::BLACK | Piece::KNIGHT)] = 0x4200000000000000;
        bitboards[Piece::to_idx(Piece::BLACK | Piece::BISHOP)] = 0x2400000000000000;
        bitboards[Piece::to_idx(Piece::BLACK | Piece::ROOK)] = 0x8100000000000000;
        bitboards[Piece::to_idx(Piece::BLACK | Piece::QUEEN)] = 0x0800000000000000;
        bitboards[Piece::to_idx(Piece::BLACK | Piece::KING)] = 0x1000000000000000;

        let mailbox = [Piece::NONE; 64];

        let mut board = Self {
            bitboards,
            occupancy,
            mailbox,
            side_to_move: Color::White,
            castling: CastlingRights::new(),
            en_passant: None,
        };

        board.build_mailbox();
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

        board.build_mailbox();
        board.build_occupancy();

        board
    }

    pub fn make_move(&mut self, mov: &Move) -> Undo {
        debug_assert!(self.occupancy[BOTH] & (1u64 << mov.from()) != 0);

        let from = mov.from();
        let to = mov.to();
        let flag = mov.flag();
        let cur_piece = self.piece_on(from);

        // Detecting captures
        let mut captured_sq = to;
        if flag == MoveFlag::EN_PASSANT {
            captured_sq = if self.side_to_move == Color::White {
                to - 8
            } else {
                to + 8
            };
        }
        let captured = self.piece_on(captured_sq);

        // Constructing undo
        let undo = Undo::new(captured, self.castling, self.en_passant);

        // Handling captures
        if captured != Piece::NONE {
            self.remove_piece(captured, captured_sq);
        }

        // Moving the piece on the board
        self.move_piece_quiet(from, to);

        // Handling Special Moves (for piece state)
        // Promotions
        if flag.is_promo() {
            // Removing the pawn, which is in the promotion square
            self.remove_piece(cur_piece, to);

            let promo_type = flag.0 & MoveFlag::PIECE_BIT;
            let color_bit = if self.side_to_move == Color::White {
                Piece::WHITE
            } else {
                Piece::BLACK
            };

            let promo_pieces = [Piece::KNIGHT, Piece::BISHOP, Piece::ROOK, Piece::QUEEN];
            let promo_piece = color_bit | promo_pieces[promo_type as usize];

            // Adding the promotion piece
            self.add_piece(promo_piece, to);
        }

        // castling
        if flag.is_castle() {
            let is_black = (cur_piece & Piece::BLACK) != 0;
            let king_pos = if is_black { BK_START_POS } else { WK_START_POS };

            if flag == MoveFlag::KING_CASTLE {
                self.move_piece_quiet(king_pos + 3, king_pos + 1);
            } else {
                self.move_piece_quiet(king_pos - 4, king_pos - 1);
            }
        }

        /////// Handling Special moves (for board state)
        // updating en_passant square
        self.en_passant = if flag == MoveFlag::DOUBLE_PUSH {
            Some(((from + to) / 2) as u8)
        } else {
            None
        };

        //// Handling castling rights
        // White king moves
        if from == WK_START_POS {
            self.castling.remove(WK | WQ);
        }

        // Black king moves
        if from == BK_START_POS {
            self.castling.remove(BK | BQ);
        }

        // If white kingside rook moved, or captured
        if from == WK_START_POS + 3 || to == WK_START_POS + 3 {
            self.castling.remove(WK);
        }

        // If white queenside rook moved, or captured
        if from == WK_START_POS - 4 || to == WK_START_POS - 4 {
            self.castling.remove(WQ);
        }

        // If black kingside rook moved, or captured
        if from == BK_START_POS + 3 || to == BK_START_POS + 3 {
            self.castling.remove(BK);
        }

        // If black queenside rook moved, or captured
        if from == BK_START_POS - 4 || to == BK_START_POS - 4 {
            self.castling.remove(BQ);
        }

        // Post move activities
        self.side_to_move = self.side_to_move.opponent();

        undo
    }

    pub fn undo_move(&mut self, mov: &Move, undo: &Undo) {
        let from = mov.from();
        let to = mov.to();
        let flag = mov.flag();
        let cur_piece = self.piece_on(to);

        debug_assert!(
            cur_piece != Piece::NONE,
            "undo_move(): piece is not on mov.to"
        );

        // move piece back
        self.move_piece_quiet(to, from);

        // handle captures
        if flag.is_capture() {
            let mut cap_sq = to;
            if flag == MoveFlag::EN_PASSANT {
                cap_sq = if Piece::get_color(undo.captured) == Piece::BLACK {
                    to - 8
                } else {
                    to + 8
                };
            }

            debug_assert!(
                undo.captured != Piece::NONE,
                "Capture without captured piece"
            );
            self.add_piece(undo.captured, cap_sq);
        }

        // Handling Promotions
        if flag.is_promo() {
            // restore pawn
            let pawn = if (cur_piece & Piece::WHITE) != 0 {
                Piece::WHITE | Piece::PAWN
            } else {
                Piece::BLACK | Piece::PAWN
            };

            // Removing the Promoted piece from mov.from
            // (cuz it got added when we try to undo the move
            self.remove_piece(cur_piece, from);
            // adding the relevent pawn on Promotion moves
            self.add_piece(pawn, from);
        }

        // castling
        if flag.is_castle() {
            if flag == MoveFlag::KING_CASTLE {
                // rook: f -> h
                self.move_piece_quiet(to - 1, to + 1);
            } else if flag == MoveFlag::QUEEN_CASTLE {
                // rook: d -> a
                self.move_piece_quiet(to + 1, to - 2);
            }
        }

        // restore state
        self.en_passant = undo.prev_en_passant_sq;
        self.castling = undo.prev_castling_rights;

        self.side_to_move = self.side_to_move.opponent();
    }

    fn is_square_atacked(&self, pos: usize, cur_color: &Color) -> bool {
        let all_occ = self.all_occ();

        let color = if cur_color == &Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let enemy_col = Piece::enemy(color);

        let (en_pawn, en_king, en_queen, en_bishop, en_rook, en_knight) = (
            self.bb(enemy_col | Piece::PAWN),
            self.bb(enemy_col | Piece::KING),
            self.bb(enemy_col | Piece::QUEEN),
            self.bb(enemy_col | Piece::BISHOP),
            self.bb(enemy_col | Piece::ROOK),
            self.bb(enemy_col | Piece::KNIGHT),
        );

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
    pub fn piece_on(&self, sq: usize) -> PieceInfo {
        return self.mailbox[sq];
    }
}

///////// Helpers
impl Board {
    fn move_piece_quiet(&mut self, from: usize, to: usize) {
        let (from_mask, to_mask) = (mask(from), mask(to));
        let piece = self.piece_on(from);

        debug_assert!(piece != Piece::NONE, "There ain't no piece in from");

        let piece_bb = self.mut_bb(piece);
        *piece_bb &= !from_mask;
        *piece_bb |= to_mask;

        self.mailbox[from] = Piece::NONE;
        self.mailbox[to] = piece;

        let color = Piece::get_color_idx(piece);
        self.occupancy[color] ^= from_mask | to_mask;
        self.occupancy[BOTH] ^= from_mask | to_mask;
    }

    fn remove_piece(&mut self, piece: PieceInfo, pos: usize) {
        let pos_mask = mask(pos);

        *self.mut_bb(piece) &= !pos_mask;
        self.mailbox[pos] = Piece::NONE;

        let color = Piece::get_color_idx(piece);
        self.occupancy[color] &= !pos_mask;
        self.occupancy[BOTH] &= !pos_mask;
    }

    fn add_piece(&mut self, piece: PieceInfo, pos: usize) {
        let pos_mask = mask(pos);

        *self.mut_bb(piece) |= pos_mask;
        self.mailbox[pos] = piece;

        let color = Piece::get_color_idx(piece);
        self.occupancy[color] |= pos_mask;
        self.occupancy[BOTH] |= pos_mask;
    }

    pub fn build_occupancy(&mut self) {
        // all white bitboards (indices 0..6)
        self.occupancy[WHITE] = self.bitboards[0..6].iter().fold(0, |acc, &bb| acc | bb);

        // all black bitboards (indices 6..12)
        self.occupancy[BLACK] = self.bitboards[6..12].iter().fold(0, |acc, &bb| acc | bb);

        self.occupancy[BOTH] = self.occupancy[WHITE] | self.occupancy[BLACK];
    }

    pub fn build_mailbox(&mut self) {
        self.mailbox = [Piece::NONE; 64];

        for (idx, &bb) in self.bitboards.iter().enumerate() {
            let mut current_bb = bb;

            while current_bb != 0 {
                if let Some(sq) = pop_lsb(&mut current_bb) {
                    self.mailbox[sq] = Piece::from_idx(idx);
                }
            }
        }
    }

    pub fn side_to_move(&self) -> Color {
        self.side_to_move
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

    #[inline(always)]
    pub fn bb(&self, piece: PieceInfo) -> u64 {
        if piece == Piece::NONE {
            return 0;
        }

        self.bitboards[Piece::to_idx(piece)]
    }

    #[inline(always)]
    pub fn mut_bb(&mut self, piece: PieceInfo) -> &mut u64 {
        debug_assert!(
            piece != Piece::NONE,
            "Attempted to get mutable bitboard of Piece::NONE"
        );

        &mut self.bitboards[Piece::to_idx(piece)]
    }
}

////////// Move generations
impl Board {
    pub fn gen_moves(&mut self) -> Vec<Move> {
        let mut moves: Vec<Move> = Vec::new();

        self.gen_king_moves(&mut moves);
        self.gen_knight_moves(&mut moves);
        self.gen_pawn_moves(&mut moves);

        let color = if self.side_to_move == Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let (bishop_bb, rook_bb, queen_bb) = (
            self.bb(color | Piece::BISHOP),
            self.bb(color | Piece::ROOK),
            self.bb(color | Piece::QUEEN),
        );

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
        let color = if self.side_to_move == Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let king = color | Piece::KING;

        let mut bb = self.bb(king);

        while let Some(from) = pop_lsb(&mut bb) {
            let to_move = &self.side_to_move;
            let occ = self.occ(to_move);

            let mut atk = KING_ATTACKS[from] & !occ;

            while let Some(to) = pop_lsb(&mut atk) {
                let flag = if (1 << to) & self.occ(&self.side_to_move.opponent()) != 0 {
                    MoveFlag::CAPTURE
                } else {
                    MoveFlag::QUIET
                };

                moves.push(Move::new(from, to, flag));
            }
        }
    }

    pub fn gen_knight_moves(&self, moves: &mut Vec<Move>) {
        let color = if self.side_to_move == Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let knight = color | Piece::KNIGHT;

        let mut bb = self.bb(knight);

        while let Some(from) = pop_lsb(&mut bb) {
            let to_move = &self.side_to_move;
            let occ = self.occ(to_move);

            let mut atk = KNIGHT_ATTACKS[from] & !occ;

            while let Some(to) = pop_lsb(&mut atk) {
                let flag = if (1 << to) & self.occ(&self.side_to_move.opponent()) != 0 {
                    MoveFlag::CAPTURE
                } else {
                    MoveFlag::QUIET
                };

                moves.push(Move::new(from, to, flag));
            }
        }
    }

    pub fn gen_pawn_moves(&self, moves: &mut Vec<Move>) {
        let (pawn, start_pos, end_pos, attacks, dir) = match self.side_to_move {
            Color::White => (
                Piece::WHITE | Piece::PAWN,
                RANK2,
                RANK8,
                WHITE_PAWN_ATTACKS,
                8i8,
            ),
            Color::Black => (
                Piece::BLACK | Piece::PAWN,
                RANK7,
                RANK1,
                BLACK_PAWN_ATTACKS,
                -8i8,
            ),
        };

        let pawn_bb = self.bb(pawn);
        let empty = !self.all_occ();

        // forward pawn moves
        let (mut single, mut double) = match Piece::get_color(pawn) {
            Piece::WHITE => {
                let single = (pawn_bb << 8) & empty;
                let double = ((pawn_bb & start_pos) << 16) & empty & (empty << 8);
                (single, double)
            }
            Piece::BLACK => {
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
                moves.push(Move::new(from, to, MoveFlag::PROMO_KNIGHT));
                moves.push(Move::new(from, to, MoveFlag::PROMO_ROOK));
                moves.push(Move::new(from, to, MoveFlag::PROMO_QUEEN));
                moves.push(Move::new(from, to, MoveFlag::PROMO_BISHOP));
            } else {
                moves.push(Move::new(from, to, MoveFlag::QUIET));
            }
        }

        // Double push
        while let Some(to) = pop_lsb(&mut double) {
            let from = (to as i8 - (2 * dir)) as usize;
            moves.push(Move::new(from, to, MoveFlag::DOUBLE_PUSH));
        }

        // Captures
        let mut bb = pawn_bb;
        let enemy = self.occ(&self.side_to_move.opponent());

        while let Some(from) = pop_lsb(&mut bb) {
            // To include en_passant sq, as there wont be any enemy there
            let target = match self.en_passant {
                Some(sq) => enemy | (1u64 << sq as usize),
                None => enemy,
            };

            let mut atk = attacks[from] & target;

            while let Some(to) = pop_lsb(&mut atk) {
                // Handling en_passant
                if let Some(sq) = self.en_passant {
                    if sq as usize == to {
                        moves.push(Move::new(from, to, MoveFlag::EN_PASSANT));
                        continue;
                    }
                }

                if (1u64 << to) & end_pos != 0 {
                    // Handling Capture Promotions
                    moves.push(Move::new(from, to, MoveFlag::PROMO_CAP_KNIGHT));
                    moves.push(Move::new(from, to, MoveFlag::PROMO_CAP_ROOK));
                    moves.push(Move::new(from, to, MoveFlag::PROMO_CAP_QUEEN));
                    moves.push(Move::new(from, to, MoveFlag::PROMO_CAP_BISHOP));
                } else {
                    moves.push(Move::new(from, to, MoveFlag::CAPTURE));
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
                        MoveFlag::CAPTURE
                    } else {
                        MoveFlag::QUIET
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
                Piece::WHITE | Piece::KING,
                Color::White,
                self.castling.white_kingside(),
                self.castling.white_queenside(),
            ),
            Color::Black => (
                Piece::BLACK | Piece::KING,
                Color::Black,
                self.castling.black_kingside(),
                self.castling.black_queenside(),
            ),
        };

        let mut king_bb = self.bb(king_piece);
        let king_pos = pop_lsb(&mut king_bb).expect("There is no King!!!");

        if ks_rights && self.can_castle_kingside(king_pos, &color) {
            moves.push(Move::new(king_pos, king_pos + 2, MoveFlag::KING_CASTLE));
        }

        if qs_rights && self.can_castle_queenside(king_pos, &color) {
            moves.push(Move::new(king_pos, king_pos - 2, MoveFlag::QUEEN_CASTLE));
        }
    }

    fn filter_illegal(&mut self, moves: Vec<Move>) -> Vec<Move> {
        let mut legal: Vec<Move> = Vec::new();

        let color = if self.side_to_move == Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let king = color | Piece::KING;

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
            Color::White => (WK_START_POS, Piece::WHITE | Piece::ROOK),
            Color::Black => (BK_START_POS, Piece::BLACK | Piece::ROOK),
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
            && self.piece_on(king_pos + 3) == rook
            && !self.is_square_atacked(king_pos + 1, &color)
            && !self.is_square_atacked(king_pos + 2, &color)
    }

    fn can_castle_queenside(&self, king_pos: usize, color: &Color) -> bool {
        let (start_pos, rook) = match color {
            Color::White => (WK_START_POS, Piece::WHITE | Piece::ROOK),
            Color::Black => (BK_START_POS, Piece::BLACK | Piece::ROOK),
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
            && self.piece_on(king_pos - 4) == rook
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
                        let piece = Piece::from_idx(i);
                        let glyph = Piece::to_char(piece);

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
