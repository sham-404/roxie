mod board;
mod square;

use crate::board::Board;
use crate::square::Square;
use std::io::{self, Write};

fn input(prompt: &str) -> String {
    let mut buffer = String::new();

    print!("{}", prompt);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut buffer).unwrap();

    return buffer.trim().to_string();
}

fn parse_move(mv: &str) -> Option<(Square, Square)> {
    let from_str = mv.get(0..2)?;
    let to_str = mv.get(2..4)?;

    let from = Square::from_str(from_str)?;
    let to = Square::from_str(to_str)?;

    Some((from, to))
}

fn main() {
    let mut board = Board::new();
    let mut buf;

    board.debug_print();
    loop {
        println!();
        buf = input("move: ");

        if buf == "quit" {
            break;
        }

        let (from, to) = if let Some((from, to)) = parse_move(&buf) {
            (from, to)
        } else {
            println!("Invalid move");
            continue;
        };

        board.move_piece(from.index(), to.index());
        board.debug_print();
    }
}
