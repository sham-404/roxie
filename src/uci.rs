use std::{
    io::{self, BufRead},
    iter::Peekable,
    str::SplitWhitespace,
};

use crate::{board::Board, engine::Engine, items::Move, perft::perft_divide, search::SearchLimits};

pub const MAX_DEPTH: u16 = 50;

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
    let mut args = commands.peekable();

    if let Some(&"perft") = args.peek() {
        args.next(); // consuming "perft"
        let depth = args.next().and_then(|val| val.parse().ok()).unwrap_or(1);
        perft_divide(&mut engine.board, depth);
        return;
    }

    let go_ctrl = GoControl::parse(&mut args);
    let limits = SearchLimits::from_go(&go_ctrl, engine.board.side_to_move());

    let data = engine.search_ids(&limits, |info| {
        info.print();
    });

    uci_print!("bestmove {}", data.best_move.to_coord());
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

#[derive(Default, Debug)]
pub struct GoControl {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u64>,
    pub depth: Option<u16>,
    pub movetime: Option<u64>,
    pub infinite: bool,
}

impl GoControl {
    fn parse(commands: &mut Peekable<&mut SplitWhitespace>) -> Self {
        let mut ctrl = Self::default();
        while let Some(arg) = commands.next() {
            match arg {
                "wtime" => ctrl.wtime = commands.next().and_then(|s| s.parse().ok()),
                "btime" => ctrl.btime = commands.next().and_then(|s| s.parse().ok()),
                "winc" => ctrl.winc = commands.next().and_then(|s| s.parse().ok()),
                "binc" => ctrl.binc = commands.next().and_then(|s| s.parse().ok()),
                "movestogo" => ctrl.movestogo = commands.next().and_then(|s| s.parse().ok()),
                "depth" => ctrl.depth = commands.next().and_then(|s| s.parse().ok()),
                "movetime" => ctrl.movetime = commands.next().and_then(|s| s.parse().ok()),
                "infinite" => ctrl.infinite = true,
                _ => {}
            }
        }
        ctrl
    }
}
