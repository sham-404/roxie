use crate::{board::Board, tt::TranspositionTable};

pub struct Engine {
    pub board: Board,
    pub tt: TranspositionTable,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::start_pos(),
            tt: TranspositionTable::new(16),
        }
    }
}
