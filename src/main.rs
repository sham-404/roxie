mod board;

use crate::board::Board;

fn main() {
    let board = Board::new();
    board.debug_print();
}
