use crate::{
    board::{Board, mask},
    r#const::{
        BLACK_PAWN_ATTACKS, KING_ATTACKS, KNIGHT_ATTACKS, RANK1, RANK2, RANK7, RANK8,
        WHITE_PAWN_ATTACKS,
    },
    engine::{CaptureHistory, Engine},
    evaluation::MG_TABLE,
    items::{Color, Move, MoveFlag, MoveList, Piece, PieceInfo},
    magics::{get_bishop_move_bits, get_rook_move_bits},
    param,
};

pub const QUIET_MV_MARGIN: i32 = 10_000_000;
#[derive(PartialEq)]
enum Phase {
    TTMove,
    GenerateTactical,
    GoodTactics, // captures and promotions
    Killers,
    CounterMove,
    GenerateQuiets,
    Quiets,
    BadCaptures,
}

impl Board {
    pub fn score_quiet_move(&self, mv: Move) -> i32 {
        let from = mv.from();
        let to = mv.to();

        // Quiets and castles
        let attacker = self.piece_on(from);
        let p_idx = Piece::to_idx(attacker);

        if let Some(mg_table) = MG_TABLE.get() {
            let pst_delta = mg_table[p_idx][to] - mg_table[p_idx][from];

            return QUIET_MV_MARGIN + pst_delta;
        }

        0
    }

    pub fn score_tactical_move(&self, mv: Move, cap_hist: &CaptureHistory) -> i32 {
        let flag = mv.flag();
        let from = mv.from();
        let to = mv.to();

        let attacker = self.piece_on(from);
        let mut victim = self.piece_on(to);

        // Promotions
        if flag.is_promo() {
            let promo_val = flag.get_promo_value() as i32;
            let score = if flag.is_capture() {
                80_000_000 + promo_val
            } else {
                60_000_000 + promo_val
            };
            return score;
        }

        // En passant
        if victim == Piece::NONE {
            debug_assert!(flag == MoveFlag::EN_PASSANT);
            victim = Piece::PAWN | Piece::enemy(Piece::get_color(attacker));
        }

        let v_val = self.get_value(victim);
        let a_val = self.get_value(attacker);

        // MVV-LVA
        let mvv_lva = (v_val * 10) - a_val;

        let see_score = self.see(&mv);

        let cap_hist_score = cap_hist.get(attacker, victim, to);

        // Max cap history score is equivalent of 2.5 centipawns
        let hist_in_centipawns = cap_hist_score >> 5;
        let adjusted_see = see_score + hist_in_centipawns;

        let base = 50_000_000;

        // trying out dynamic formula for scoring
        let final_score = base + (adjusted_see * 100) + mvv_lva + (cap_hist_score * 2);

        // Now good capture is not only based on see, but it also takes capture history
        // into consideration for assigning a move as good capture, even see is negative
        if see_score >= 0 || adjusted_see >= 0 {
            // it is a good capture
            final_score
        } else {
            final_score - 49_000_000
        }
    }

    pub fn is_pseudo_legal_mv(&self, mv: Move) -> bool {
        if mv == Move::NULL {
            return false;
        }

        let from = mv.from();

        // is the piece is present on "from"
        let piece = self.piece_on(from);
        if piece == Piece::NONE {
            return false;
        }

        // is the present piece is ours
        if Piece::get_color_idx(piece) != self.side_to_move().val() {
            return false;
        }

        let to_move = self.side_to_move();
        let own_occ = self.occ(&to_move);
        let opp_occ = self.occ(&to_move.opponent());
        let to = mv.to();
        let to_bb = mask(to);

        // is the to square is occupied by our piece?
        if own_occ & to_bb != 0 {
            return false;
        }

        // if it is a capture, is the to square is empty? and if
        // it is not, is the square is occupied by enemy piece
        let flag = mv.flag();
        if flag != MoveFlag::EN_PASSANT && flag.is_capture() != ((opp_occ & to_bb) != 0) {
            return false;
        }

        // From this part, we assume that the piece is present in 'from' square,
        // which is our piece to move, and the 'to' square is not occupied by
        // our piece, also for captures, if it is a capture, then the 'to' square
        // contains enemy piece
        //
        // So we will be proceeding by considering those validations and will not
        // be verified again in the later part (exept if i am a dumbo and verify it again)
        match Piece::get_type(piece) {
            Piece::PAWN => {
                let (start_rank, promo_rank, attacks, single_push_bb, double_push_bb) =
                    match to_move {
                        Color::White => (
                            RANK2,
                            RANK8,
                            WHITE_PAWN_ATTACKS[from],
                            mask(from) << 8,
                            mask(from) << 16,
                        ),
                        Color::Black => (
                            RANK7,
                            RANK1,
                            BLACK_PAWN_ATTACKS[from],
                            mask(from) >> 8,
                            mask(from) >> 16,
                        ),
                    };

                let is_promo_rank = (to_bb & promo_rank) != 0;

                if flag == MoveFlag::EN_PASSANT {
                    return self.get_en_passant() == Some(to as u8) && (attacks & to_bb) != 0;
                }

                if flag.is_capture() {
                    return (attacks & to_bb) != 0 && (flag.is_promo() == is_promo_rank);
                }

                if flag == MoveFlag::DOUBLE_PUSH {
                    // Must start on start_rank, land 2 squares ahead, and have the middle square empty
                    return (mask(from) & start_rank) != 0
                        && to_bb == double_push_bb
                        && (single_push_bb & self.all_occ()) == 0;
                }

                // Must be a single push
                return to_bb == single_push_bb && (flag.is_promo() == is_promo_rank);
            }

            Piece::KNIGHT => {
                return flag.is_normal() && (KNIGHT_ATTACKS[from] & to_bb) != 0;
            }

            Piece::BISHOP => {
                return flag.is_normal()
                    && ((get_bishop_move_bits(from, self.all_occ()) & !own_occ) & to_bb) != 0;
            }

            Piece::ROOK => {
                return flag.is_normal()
                    && ((get_rook_move_bits(from, self.all_occ()) & !own_occ) & to_bb) != 0;
            }

            Piece::QUEEN => {
                let all_occ = self.all_occ();
                let queen_move_bits = (get_bishop_move_bits(from, all_occ)
                    | get_rook_move_bits(from, all_occ))
                    & !own_occ;

                return flag.is_normal() && (queen_move_bits & to_bb) != 0;
            }

            Piece::KING => match flag {
                MoveFlag::KING_CASTLE => {
                    let has_rights = match to_move {
                        Color::White => self.get_castling_rights().white_kingside(),
                        Color::Black => self.get_castling_rights().black_kingside(),
                    };
                    return to == from + 2
                        && has_rights
                        && self.can_castle_kingside(from, &to_move);
                }
                MoveFlag::QUEEN_CASTLE => {
                    let has_rights = match to_move {
                        Color::White => self.get_castling_rights().white_queenside(),
                        Color::Black => self.get_castling_rights().black_queenside(),
                    };
                    return to + 2 == from
                        && has_rights
                        && self.can_castle_queenside(from, &to_move);
                }
                MoveFlag::QUIET | MoveFlag::CAPTURE => {
                    let to_bb = mask(to);
                    let attacks = KING_ATTACKS[from] & !self.occ(&to_move);
                    if (attacks & to_bb) == 0 {
                        return false;
                    }
                    let is_capture = (self.occ(&to_move.opponent()) & to_bb) != 0;
                    return (flag == MoveFlag::CAPTURE) == is_capture;
                }
                _ => return false,
            },
            _ => unreachable!("how did you reach here brotha?"),
        }
    }
}

pub struct MovePicker {
    phase: Phase,
    tt_move: Move,
    killers: [Move; 2],
    prev_mv: Move,
    prev_piece: PieceInfo,
    prev2_mv: Move,
    prev2_piece: PieceInfo,
    prev4_mv: Move,
    prev4_piece: PieceInfo,
    qsearch_picker: bool,

    moves: MoveList,
    mv_idx: usize,

    bad_captures: MoveList,
}

impl MovePicker {
    pub fn new(
        tt_move: Move,
        killers: [Move; 2],
        prev_mv: Move,
        prev_piece: PieceInfo,
        prev2_mv: Move,
        prev2_piece: PieceInfo,
        prev4_mv: Move,
        prev4_piece: PieceInfo,
        qsearch_picker: bool,
    ) -> Self {
        Self {
            phase: Phase::TTMove,
            tt_move,
            killers,
            prev_mv,
            prev_piece,
            prev2_mv,
            prev2_piece,
            prev4_mv,
            prev4_piece,
            qsearch_picker,
            moves: MoveList::new(),
            mv_idx: 0,
            bad_captures: MoveList::new(),
        }
    }

    pub fn skip_quiets(&mut self) {
        if self.phase == Phase::Quiets {
            self.phase = Phase::BadCaptures;
        }
    }
}

impl Engine {
    pub fn pick_next_mv(&mut self, picker: &mut MovePicker) -> Option<Move> {
        loop {
            match picker.phase {
                Phase::TTMove => {
                    picker.phase = Phase::GenerateTactical;
                    if picker.tt_move != Move::NULL && self.board.is_pseudo_legal_mv(picker.tt_move)
                    {
                        return Some(picker.tt_move);
                    }
                }

                Phase::GenerateTactical => {
                    picker.moves = self.board.gen_tactical_moves();

                    for i in 0..picker.moves.len() {
                        let score = self
                            .board
                            .score_tactical_move(picker.moves.get(i), &self.capture_history);
                        picker.moves.score[i] = score;
                    }

                    picker.mv_idx = 0;
                    picker.phase = Phase::GoodTactics;
                }

                Phase::GoodTactics => {
                    while picker.mv_idx < picker.moves.len() {
                        let mv = picker.moves.pick_move(picker.mv_idx);
                        let score = picker.moves.score[picker.mv_idx]; // score is moved to the mv_idx
                        picker.mv_idx += 1;

                        if mv == picker.tt_move {
                            continue;
                        }

                        if score < QUIET_MV_MARGIN {
                            // it is a bad capture then
                            picker.bad_captures.push_with_score(mv, score);
                            continue;
                        }

                        return Some(mv);
                    }

                    // all captures were processed
                    if picker.qsearch_picker {
                        // skipping the bad captures too
                        return None;
                    }

                    picker.phase = Phase::Killers;
                    picker.mv_idx = 0;
                }

                Phase::Killers => {
                    while picker.mv_idx < picker.killers.len() {
                        let killer = picker.killers[picker.mv_idx];
                        picker.mv_idx += 1;

                        if killer != Move::NULL
                            && killer != picker.tt_move
                            && self.board.is_pseudo_legal_mv(killer)
                        {
                            return Some(killer);
                        }
                    }

                    picker.phase = Phase::CounterMove;
                    picker.mv_idx = 0;
                }

                Phase::CounterMove => {
                    picker.phase = Phase::GenerateQuiets;
                    let counter_mv = self.counter_moves.get(picker.prev_mv);
                    if ![
                        picker.tt_move,
                        picker.killers[0],
                        picker.killers[1],
                        Move::NULL,
                    ]
                    .contains(&counter_mv)
                        && self.board.is_pseudo_legal_mv(counter_mv)
                    {
                        return Some(counter_mv);
                    }
                }

                Phase::GenerateQuiets => {
                    picker.mv_idx = 0;
                    picker.moves = self.board.gen_quiet_moves();

                    for i in 0..picker.moves.len() {
                        let mv = picker.moves.get(i);
                        let mut score = self.board.score_quiet_move(mv);
                        let cur_piece = self.board.piece_on(mv.from());

                        score +=
                            self.history
                                .get(self.board.side_to_move().val(), mv.from(), mv.to())
                                << param!(HIST_SHIFT);
                        score += (self.continuation_history.get(
                            picker.prev_mv,
                            picker.prev_piece,
                            mv,
                            cur_piece,
                        ) as i32)
                            << param!(CONT_HIST_SHIFT_1);
                        score += (self.continuation_history_2.get(
                            picker.prev2_mv,
                            picker.prev2_piece,
                            mv,
                            cur_piece,
                        ) as i32)
                            << param!(CONT_HIST_SHIFT_2);
                        score += (self.continuation_history_4.get(
                            picker.prev4_mv,
                            picker.prev4_piece,
                            mv,
                            cur_piece,
                        ) as i32)
                            << param!(CONT_HIST_SHIFT_4);
                        picker.moves.score[i] = score;
                    }

                    picker.phase = Phase::Quiets;
                }

                Phase::Quiets => {
                    while picker.mv_idx < picker.moves.len() {
                        let mv = picker.moves.pick_move(picker.mv_idx);
                        let counter_mv = self.counter_moves.get(picker.prev_mv);
                        picker.mv_idx += 1;

                        if [
                            picker.tt_move,
                            picker.killers[0],
                            picker.killers[1],
                            counter_mv,
                            Move::NULL,
                        ]
                        .contains(&mv)
                        {
                            continue;
                        }

                        return Some(mv);
                    }

                    picker.mv_idx = 0;
                    picker.phase = Phase::BadCaptures;
                }

                Phase::BadCaptures => {
                    // scoring is already done so scoring is not needed
                    if picker.mv_idx < picker.bad_captures.len() {
                        let mv = picker.bad_captures.pick_move(picker.mv_idx);
                        picker.mv_idx += 1;

                        return Some(mv);
                    }
                    picker.bad_captures.clear();

                    return None;
                }
            }
        }
    }
}
