use crate::{
    board::Board,
    r#const::MAX_PLY,
    items::{Move, Piece},
    network::{EvalBuf, HL1},
    search::MAX_HISTORY,
    tt::TranspositionTable,
    uci_print,
};

pub struct Engine {
    pub board: Board,
    pub tt: TranspositionTable,
    pub history: [[[i32; 64]; 64]; 2],
    pub continuation_history: ContinuationHistory,
    pub counter_moves: CountermoveTable,
    pub eval_history: EvalHistory,
    pub killers: [[Move; 2]; MAX_PLY],
    pub eval_buf: EvalBuf,
    pub accumulators: [[[i32; HL1]; 2]; MAX_PLY],
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::start_pos(),
            tt: TranspositionTable::new(25),
            history: [[[0; 64]; 64]; 2],
            continuation_history: ContinuationHistory::new(),
            counter_moves: CountermoveTable::new(),
            eval_history: EvalHistory::new(),
            killers: [[Move::NULL; 2]; MAX_PLY],
            eval_buf: EvalBuf::new(),
            accumulators: [[[0i32; HL1]; 2]; MAX_PLY],
        }
    }

    #[inline]
    pub fn info(&self) {
        uci_print!("Roxie v{}\n", env!("CARGO_PKG_VERSION"));
        self.tt.info();
    }
}

pub struct CountermoveTable {
    pub table: [[Move; 64]; 64],
}

impl CountermoveTable {
    pub fn new() -> Self {
        Self {
            table: [[Move::NULL; 64]; 64],
        }
    }

    pub fn store(&mut self, prev_mv: Move, cur_mv: Move) {
        if prev_mv == Move::NULL {
            return;
        }
        self.table[prev_mv.from()][prev_mv.to()] = cur_mv;
    }

    pub fn get(&self, prev_mv: Move) -> Move {
        self.table[prev_mv.from()][prev_mv.to()]
    }
}

pub struct ContinuationHistory {
    // table[piece_idx * 64 + prev_mv.to][piece_idx * 64 + cur_mv.to]
    pub table: Box<[[i16; 768]; 768]>,
}

impl ContinuationHistory {
    pub fn new() -> Self {
        let table = vec![[0i16; 768]; 768];
        Self {
            table: table.into_boxed_slice().try_into().unwrap(),
        }
    }

    #[inline]
    pub fn get_after_mv(&self, board: &Board, prev_mv: Move, cur_mv: Move) -> i16 {
        if prev_mv == Move::NULL || cur_mv == Move::NULL {
            return 0;
        }

        let prev_idx = Piece::to_idx(board.piece_on(prev_mv.to())) * 64 + prev_mv.to();
        let cur_idx = Piece::to_idx(board.piece_on(cur_mv.to())) * 64 + cur_mv.to();

        self.table[prev_idx][cur_idx]
    }

    #[inline]
    pub fn get(&self, board: &Board, prev_mv: Move, cur_mv: Move) -> i16 {
        debug_assert_ne!(cur_mv, Move::NULL); // cur_mv is expected to be a valid move
        if prev_mv == Move::NULL {
            return 0;
        }

        let prev_idx = Piece::to_idx(board.piece_on(prev_mv.to())) * 64 + prev_mv.to();
        let cur_idx = Piece::to_idx(board.piece_on(cur_mv.from())) * 64 + cur_mv.to();

        self.table[prev_idx][cur_idx]
    }

    #[inline]
    pub fn update(&mut self, board: &Board, prev_mv: Move, cur_mv: Move, bonus: i32) {
        debug_assert_ne!(cur_mv, Move::NULL); // cur_mv is expected to be a valid move
        if prev_mv == Move::NULL {
            return;
        }

        // index = piece_idx * 64 + mv.to
        let prev_idx = Piece::to_idx(board.piece_on(prev_mv.to())) * 64 + prev_mv.to();
        let cur_idx = Piece::to_idx(board.piece_on(cur_mv.from())) * 64 + cur_mv.to();

        let cur_val = self.table[prev_idx][cur_idx];
        let new_val = (cur_val as i32) + bonus - (cur_val as i32 * bonus.abs() / MAX_HISTORY);

        self.table[prev_idx][cur_idx] = new_val.clamp(-MAX_HISTORY, MAX_HISTORY) as i16;
    }
}

pub struct EvalHistory {
    evals: [i16; MAX_PLY],
    checks: [bool; MAX_PLY],
}

impl EvalHistory {
    pub fn new() -> Self {
        Self {
            evals: [0; MAX_PLY],
            checks: [false; MAX_PLY],
        }
    }

    #[inline]
    pub fn store(&mut self, score: i16, in_check: bool, ply: usize) {
        self.evals[ply] = score;
        self.checks[ply] = in_check;
    }

    #[inline]
    pub fn is_improving(&self, current_eval: i16, in_check: bool, ply: usize) -> bool {
        if in_check {
            return false;
        }

        if ply >= 2 {
            let mut past_ply = ply - 2;

            // If we were in check last turn, then check the next previous round, (ply - 4)
            if past_ply >= 2 && self.checks[past_ply] {
                past_ply -= 2;
            }

            return current_eval > self.evals[past_ply];
        }

        true // true for first 2 plies btw
    }

    #[inline]
    pub fn clear(&mut self) {
        self.evals = [0; MAX_PLY];
        self.checks = [false; MAX_PLY];
    }
}
