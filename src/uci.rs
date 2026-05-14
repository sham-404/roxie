use std::{
    io::{self, BufRead},
    iter::Peekable,
    str::SplitWhitespace,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
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

pub struct UCI {
    engine: Arc<Mutex<Engine>>,
    stop_signal: Arc<AtomicBool>,
    search_handle: Option<JoinHandle<()>>,
}

impl UCI {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(Engine::new())),
            stop_signal: Arc::new(AtomicBool::new(false)),
            search_handle: None,
        }
    }

    pub fn uci_loop(&mut self) {
        let stdin = io::stdin();

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

                    "ucinewgame" => {
                        self.stop_search();
                        let mut engine_guard = self.engine.lock().unwrap();
                        *engine_guard = Engine::new()
                    }

                    "position" => {
                        self.stop_search();
                        self.handle_position(&mut words)
                    }

                    "stop" => self.stop_signal.store(true, Ordering::Relaxed),

                    "go" => {
                        self.stop_search();
                        self.handle_go(&mut words)
                    }

                    "quit" => {
                        self.stop_search();
                        break;
                    }

                    _ => {}
                }
            }
        }
    }

    fn handle_go<'a>(&mut self, commands: &mut SplitWhitespace<'a>) {
        let mut args = commands.peekable();

        if let Some(&"perft") = args.peek() {
            args.next(); // consuming "perft"
            let depth = args.next().and_then(|val| val.parse().ok()).unwrap_or(1);
            let mut engine_guard = self.engine.lock().unwrap();
            perft_divide(&mut engine_guard.board, depth);
            return;
        }

        let go_ctrl = GoControl::parse(&mut args);
        let stm = {
            let engine_guard = self.engine.lock().unwrap();
            engine_guard.board.side_to_move()
        };

        let mut limits = SearchLimits::from_go(&go_ctrl, stm);
        limits.stop_signal = Arc::clone(&self.stop_signal);
        self.stop_signal.store(false, Ordering::Relaxed);

        let thread_engine = Arc::clone(&self.engine);

        self.search_handle = Some(thread::spawn(move || {
            let mut engine_guard = thread_engine.lock().unwrap();
            let data = engine_guard.search_ids(&limits, |info| {
                info.print();
            });

            uci_print!("bestmove {}", data.best_move.to_coord());
        }));
    }

    fn handle_position<'a>(&self, commands: &mut SplitWhitespace<'a>) {
        let mut engine = self.engine.lock().unwrap();
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

    fn stop_search(&mut self) {
        if let Some(handle) = self.search_handle.take() {
            self.stop_signal.store(true, Ordering::Relaxed);
            let _ = handle.join();
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
