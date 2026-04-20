mod board;

use crate::board::Board;

fn main() {
    let board = Board::new();
    board.print_board();
}
