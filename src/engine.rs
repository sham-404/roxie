use crate::{board::Board, tt::TranspositionTable};

pub struct Engine {
    pub board: Board,
    pub tt: TranspositionTable,
    pub history: Vec<Vec<Vec<u16>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::start_pos(),
            tt: TranspositionTable::new(16),
            history: vec![vec![vec![0; 64]; 64]; 2],
        }
    }
}
