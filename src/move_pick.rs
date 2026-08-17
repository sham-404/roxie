use crate::{
    board::Board,
    engine::Engine,
    evaluation::MG_TABLE,
    items::{Move, MoveFlag, MoveList, Piece},
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

    pub fn score_tactical_move(&self, mv: Move) -> i32 {
        let flag = mv.flag();
        let from = mv.from();
        let to = mv.to();

        let attacker = self.piece_on(from);
        let victim = self.piece_on(to);

        // promotions updation
        if flag.is_promo() {
            if !flag.is_capture() {
                // normal promotion
                return 60_000_000 + flag.get_promo_value() as i32;
            }

            // capture promotion
            return 80_000_000 + flag.get_promo_value() as i32;
        }

        // En passant
        if victim == Piece::NONE {
            debug_assert!(flag == MoveFlag::EN_PASSANT);
            if flag != MoveFlag::EN_PASSANT {
                println!("{}", mv.to_coord());
            }
            return 50_000_000;
        }

        let v_val = self.get_value(victim);
        let a_val = self.get_value(attacker);

        // MVV-LVA
        let mvv_lva = (v_val * 10) - a_val;

        let see_score = self.see(&mv);

        // Winning capture
        if see_score > 0 {
            return 70_000_000 + mvv_lva + see_score;
        }

        // Equal capture
        if see_score == 0 {
            return 50_000_000 + mvv_lva + see_score;
        }

        // Losing capture
        return 1_000_000 + mvv_lva + see_score;
    }

    pub fn is_pseudo_legal_mv(&self, mv: Move) -> bool {
        if mv == Move::NULL {
            return false;
        }

        let from = mv.from();
        // let to = mv.to();
        // let flag = mv.flag();

        // is the piece is present on "from"
        let piece = self.piece_on(from);
        if piece == Piece::NONE {
            return false;
        }

        // is the present piece is ours
        if Piece::get_color_idx(piece) != self.side_to_move().val() {
            return false;
        }

        // generating the pseudo legal for the piece type on "from"
        // and compare it with the move

        let mut moves = MoveList::new();
        match Piece::get_type(piece) {
            Piece::PAWN => self.gen_pawn_moves_from_sq(from, &mut moves),
            Piece::KNIGHT => self.gen_knight_moves_from_sq(from, &mut moves),
            Piece::BISHOP => self.gen_bishop_moves_from_sq(from, &mut moves),
            Piece::ROOK => self.gen_rook_moves_from_sq(from, &mut moves),
            Piece::QUEEN => self.gen_queen_moves_from_sq(from, &mut moves),
            Piece::KING => self.gen_king_moves_from_sq(from, &mut moves),
            _ => unreachable!("how did you reach here brotha?"),
        }

        if !moves.as_slice().contains(&mv) {
            return false;
        }

        true
    }
}

pub struct MovePicker {
    phase: Phase,
    tt_move: Move,
    killers: [Move; 2],
    prev_mv: Move,
    qsearch_picker: bool,

    moves: MoveList,
    mv_idx: usize,

    bad_captures: MoveList,
}

impl MovePicker {
    pub fn new(tt_move: Move, killers: [Move; 2], prev_mv: Move, qsearch_picker: bool) -> Self {
        Self {
            phase: Phase::TTMove,
            tt_move,
            killers,
            prev_mv,
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
                        let score = self.board.score_tactical_move(picker.moves.get(i));
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

                        score +=
                            self.history
                                .get(self.board.side_to_move().val(), mv.from(), mv.to())
                                << 2;
                        score += (self
                            .continuation_history
                            .get(&self.board, picker.prev_mv, mv)
                            << 4) as i32;
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
                    while picker.mv_idx < picker.bad_captures.len() {
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
