use std::{
    io::{self, BufRead},
    str::SplitWhitespace,
    time::Duration,
};

use crate::{board::Board, engine::Engine, items::Move, perft::perft_divide};

const MAX_DEPTH: u16 = 50;

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
    let mut engine = Engine::new();

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

                "ucinewgame" => engine = Engine::new(),

                "position" => handle_position(&mut words, &mut engine),

                "go" => handle_go(&mut words, &mut engine),

                "quit" => break,

                _ => {}
            }
        }
    }
}

fn handle_go<'a>(commands: &mut SplitWhitespace<'a>, engine: &mut Engine) {
    if let Some(cmd) = commands.next() {
        match cmd {
            "perft" => {
                if let Ok(depth) = commands.next().unwrap().parse::<u32>() {
                    perft_divide(&mut engine.board, depth);
                }
            }

            "movetime" => {
                let time_limit: u64 = commands.next().unwrap_or("1").parse().unwrap();

                let data =
                    engine.search_ids(MAX_DEPTH, Some(Duration::from_millis(time_limit)), |info| {
                        info.print();
                    });

                let coord = data.best_move.to_coord();

                uci_print!("bestmove {}", coord);
            }

            "depth" => {
                let depth: u16 = commands.next().unwrap_or("1").parse().unwrap();

                let data = engine.search_ids(depth, None, |info| {
                    info.print();
                });

                let coord = data.best_move.to_coord();

                uci_print!("bestmove {}", coord);
            }
            _ => {} // need to implement infinite search
        }
    } else {
        let data = engine.search_ids(1, None, |info| {
            info.print();
        });
        let coord = data.best_move.to_coord();
        uci_print!("bestmove {}", coord);
    }
}

fn handle_position<'a>(commands: &mut SplitWhitespace<'a>, engine: &mut Engine) {
    if let Some(cmd) = commands.next() {
        match cmd {
            "startpos" => {
                engine.board = Board::start_pos();

                if let Some("moves") = commands.next() {
                    for mv_str in commands {
                        let mv = Move::from_uci(mv_str, &mut engine.board);
                        engine.board.make_move(&mv);
                    }
                }
            }

            "fen" => {
                let fen_parts: Vec<&str> = commands.by_ref().take(6).collect();
                let fen = fen_parts.join(" ");

                engine.board = Board::load_fen(&fen);

                if let Some("moves") = commands.next() {
                    for mv_str in commands {
                        let mv = Move::from_uci(mv_str, &mut engine.board);
                        engine.board.make_move(&mv);
                    }
                }
            }

            _ => {}
        }
    }
}
