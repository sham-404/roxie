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

use crate::{
    board::Board,
    r#const::MAX_PLY,
    engine::{Engine, SharedState},
    items::Move,
    network::NETWORK,
    perft::perft_divide,
    search::SearchLimits,
    tt::TranspositionTable,
};

pub const MAX_DEPTH: u16 = MAX_PLY as u16;

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
    engine: Arc<Mutex<Box<Engine>>>,
    stop_signal: Arc<AtomicBool>,
    search_handle: Option<JoinHandle<()>>,

    num_threads: u16,
    debug: bool,
    stats: bool,
}

impl UCI {
    pub fn new() -> Self {
        let engine = Arc::new(Mutex::new(Box::new(Engine::new())));
        let stop_signal = engine.lock().unwrap().shared.abort.clone();
        Self {
            engine,
            stop_signal,
            search_handle: None,
            num_threads: 1,
            debug: false,
            stats: false,
        }
    }

    fn options() {
        uci_print!("option name Hash type spin default 16 min 1 max 1048576");
        uci_print!("option name Clear Hash type button");
        uci_print!("option name Threads type spin default 1 min 1 max 1024");
    }

    pub fn uci_loop(&mut self) {
        self.engine.lock().unwrap().info();

        let stdin = io::stdin();

        for line in stdin.lock().lines() {
            let line = line.unwrap();
            let mut words = line.trim().split_whitespace();

            if let Some(cmd) = words.next() {
                match cmd {
                    "uci" => {
                        uci_print!("id name Roxie {}", env!("CARGO_PKG_VERSION"));
                        uci_print!("id author Sham Sujith");
                        uci_print!();

                        UCI::options();

                        uci_print!("uciok");
                    }

                    "debug" => {
                        let cmd = words.next();
                        if cmd == Some("on") {
                            uci_print!("Executing in debug mode");
                            self.debug = true;
                        } else if cmd == Some("off") {
                            uci_print!("Executing in normal mode");
                            self.debug = false;
                        }
                    }

                    "eval" => {
                        let eval = NETWORK
                            .get()
                            .unwrap()
                            .evaluate(&self.engine.lock().unwrap().board);
                        uci_print!("Raw Evaluation Score: {} cp", eval);
                    }

                    "stats" => {
                        let cmd = words.next();
                        if cmd == Some("on") {
                            uci_print!("Turning on stats");
                            self.stats = true;
                        } else if cmd == Some("off") {
                            uci_print!("Turning off stats");
                            self.stats = false;
                        }
                    }

                    "isready" => {
                        uci_print!("readyok");
                    }

                    "ucinewgame" => {
                        self.stop_search();
                        self.engine.lock().unwrap().reset();
                    }

                    "setoption" => self.handle_setoption(&mut words),

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
        let (stm, game_phase) = {
            let engine_guard = self.engine.lock().unwrap();
            (
                engine_guard.board.side_to_move(),
                engine_guard.board.get_game_phase(),
            )
        };

        let limits = SearchLimits::from_go(&go_ctrl, stm, game_phase);

        let thread_engine = Arc::clone(&self.engine);
        let debug = self.debug;
        let stats = self.stats;
        let num_threads = self.num_threads;

        self.search_handle = Some(thread::spawn(move || {
            // root engine, handles time and actual search (idx = 0)
            let mut base_engine = thread_engine.lock().unwrap();

            let data = thread::scope(|s| {
                // worker threads (idx = 1..num_threads)
                for id in 1..num_threads {
                    let mut helper_engine = base_engine.child(id);

                    s.spawn(move || {
                        helper_engine.search_ids(&limits, |_| {});
                    });
                }

                // main engine search (idx = 0)
                let best_info = base_engine.search_ids(&limits, |info| {
                    info.print();
                    if debug {
                        info.stats.describe();
                    }
                });

                base_engine.shared.abort.store(true, Ordering::Relaxed);

                best_info
            });

            if stats {
                data.stats.print_stats();
            }

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
                            if let Some(mv) = Move::from_uci(mv_str, &mut engine.board) {
                                engine.board.make_move(&mv);
                            } else {
                                uci_print!("info string ignoring illegal move: {}", mv_str);
                                return;
                            }
                        }
                    }
                }

                "fen" => {
                    let mut fen_parts = Vec::new();
                    let mut has_moves = false;

                    // Collecting parts till it ends, or if we have "moves" command
                    while let Some(part) = commands.next() {
                        if part == "moves" {
                            has_moves = true;
                            break;
                        }
                        fen_parts.push(part);
                    }

                    let fen = fen_parts.join(" ");
                    engine.board = Board::load_fen(&fen);

                    // if we broke out of the loop because we hit "moves",
                    // the remaining items in `commands` are the actual moves.
                    if has_moves {
                        for mv_str in commands {
                            if let Some(mv) = Move::from_uci(mv_str, &mut engine.board) {
                                engine.board.make_move(&mv);
                            } else {
                                uci_print!("info string ignoring illegal move: {}", mv_str);
                                return;
                            }
                        }
                    }
                }

                _ => {}
            }
        }
    }

    fn handle_setoption<'a>(&mut self, commands: &mut SplitWhitespace<'a>) {
        // Stopping the search if it is running
        self.stop_search();

        // checking whether the next arg is "name"
        if commands.next() != Some("name") {
            uci_print!("Incomplete setoption parameters");
            return;
        }

        match commands.next() {
            //// Option: Hash
            Some(cmd) if cmd.eq_ignore_ascii_case("hash") => {
                // checking whether the next arg is "value"
                if commands.next() != Some("value") {
                    uci_print!("Incomplete setoption parameters");
                    return;
                }

                let val = match commands.next().and_then(|v| v.parse::<usize>().ok()) {
                    Some(val) => val,
                    None => {
                        uci_print!("Invalid option");
                        return;
                    }
                }
                .clamp(1, 1_048_576);

                let mut engine = self.engine.lock().unwrap();
                engine.shared = SharedState {
                    tt: Arc::new(TranspositionTable::new(val)),
                    abort: Arc::clone(&engine.shared.abort),
                    nodes: Arc::clone(&engine.shared.nodes),
                };

                engine.shared.tt.info();
                return;
            }

            //// Option: Threads
            Some(cmd) if cmd.eq_ignore_ascii_case("threads") => {
                // checking whether the next arg is "value"
                if commands.next() != Some("value") {
                    uci_print!("Incomplete setoption parameters");
                    return;
                }

                self.num_threads = match commands.next().and_then(|v| v.parse::<u16>().ok()) {
                    Some(val) => val,
                    None => {
                        uci_print!("Invalid option");
                        return;
                    }
                }
                .clamp(1, 1024);

                return;
            }

            //// Option: Clear Hash
            Some(cmd) if cmd.eq_ignore_ascii_case("clear") => match commands.next() {
                Some(cmd) if cmd.eq_ignore_ascii_case("hash") => {
                    let engine = self.engine.lock().unwrap();
                    engine.shared.tt.clear();
                    return;
                }

                Some(_) | None => {
                    uci_print!("Invalid option type");
                    return;
                }
            },

            Some(_) => {
                uci_print!("Invalid option");
                return;
            }

            None => {
                uci_print!("No options specified");
                return;
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
    pub nodes: Option<u64>,
    pub mate: Option<u64>,
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
                "nodes" => ctrl.nodes = commands.next().and_then(|s| s.parse().ok()),
                "mate" => ctrl.mate = commands.next().and_then(|s| s.parse().ok()),
                "movetime" => ctrl.movetime = commands.next().and_then(|s| s.parse().ok()),
                "infinite" => ctrl.infinite = true,
                _ => {}
            }
        }
        ctrl
    }
}
