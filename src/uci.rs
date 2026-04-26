use std::{
    io::{self, BufRead},
    str::SplitWhitespace,
};

use crate::{board::Board, items::Move, search::find_best_move};

#[macro_export]
macro_rules! uci_print {
    ($($arg:tt)*) => {{
        use std::io::{self, Write};
        let mut stdout = io::stdout();
        writeln!(stdout, $($arg)*).unwrap();
        stdout.flush().unwrap();
    }};
}

pub fn uci_loop() {
    let stdin = io::stdin();
    let mut board = Board::new();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let mut words = line.trim().split_whitespace();

        if let Some(cmd) = words.next() {
            match cmd {
                "uci" => {
                    uci_print!("id name Roxie {}", env!("CARGO_PKG_VERSION"));
                    uci_print!("id author sham-404");
                    uci_print!("uciok");
                }

                "isready" => {
                    uci_print!("readyok");
                }

                "ucinewgame" => board = Board::new(),

                "position" => handle_position(&mut words, &mut board),

                "go" => uci_print!("bestmove {}", find_best_move(&mut board).to_coord()),

                "quit" => break,

                _ => {}
            }
        }
    }
}

fn handle_position<'a>(commands: &mut SplitWhitespace<'a>, board: &mut Board) {
    if let Some(cmd) = commands.next() {
        match cmd {
            "startpos" => {
                *board = Board::start_pos();

                if let Some("moves") = commands.next() {
                    for mv_str in commands {
                        let mv = Move::from_uci(mv_str, board);
                        board.make_move(&mv);
                    }
                }
            }

            "fen" => {
                let fen_parts: Vec<&str> = commands.by_ref().take(6).collect();
                let fen = fen_parts.join(" ");

                *board = Board::load_fen(&fen);

                if let Some("moves") = commands.next() {
                    for mv_str in commands {
                        let mv = Move::from_uci(mv_str, board);
                        board.make_move(&mv);
                    }
                }
            }

            _ => {}
        }
    }
}
