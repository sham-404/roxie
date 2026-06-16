use crate::{
    board::Board, r#const::MAX_PLY, items::Move, network::EvalBuf, tt::TranspositionTable,
};

pub struct Engine {
    pub board: Board,
    pub tt: TranspositionTable,
    pub history: [[[i32; 64]; 64]; 2],
    pub killers: [[Move; 2]; MAX_PLY],
    pub eval_buf: EvalBuf,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::start_pos(),
            tt: TranspositionTable::new(16),
            history: [[[0; 64]; 64]; 2],
            killers: [[Move::NULL; 2]; MAX_PLY],
            eval_buf: EvalBuf::new(),
        }
    }
}
