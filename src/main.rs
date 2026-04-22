mod board;
mod items;
mod square;

use crate::board::Board;
use crate::square::Square;
use std::{
    io::{self, Write},
    str,
};

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

    println!("Loop started");
    loop {
        println!();
        let buf = input("");

        let mut parts = buf.split_whitespace();

        let command = if let Some(cmd) = parts.next() {
            cmd
        } else {
            continue;
        };

        match command {
            "quit" => break,

            "print" => {
                board.print_many(vec![board.render_board()]);
            }

            "move" | "mv" => {
                let args = parts.collect::<Vec<_>>().join("");

                let (from, to) = if let Some((from, to)) = parse_move(&args) {
                    (from, to)
                } else {
                    println!("Invalid notation");
                    continue;
                };

                let moves = board.gen_moves();

                if moves
                    .iter()
                    .any(|mv| mv.from == from.index() && mv.to == to.index())
                {
                    board.move_piece(from.index(), to.index());

                    let bb = board
                        .gen_moves()
                        .iter()
                        .fold(0u64, |acc, mv| acc | (1u64 << mv.to));
                    board.print_many(vec![board.render_board(), board.render_bitboard(bb)]);
                } else {
                    println!("Invalid move")
                }
            }

            _ => {
                println!("Invalid command");
            }
        }
    }
}
