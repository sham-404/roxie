use crate::{
    board::Board,
    r#const::MAX_PLY,
    items::Move,
    network::{EvalBuf, HL1},
    tt::TranspositionTable,
};

pub struct Engine {
    pub board: Board,
    pub tt: TranspositionTable,
    pub history: [[[i32; 64]; 64]; 2],
    pub counter_moves: CountermoveTable,
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
            counter_moves: CountermoveTable::new(),
            killers: [[Move::NULL; 2]; MAX_PLY],
            eval_buf: EvalBuf::new(),
            accumulators: [[[0i32; HL1]; 2]; MAX_PLY],
        }
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
