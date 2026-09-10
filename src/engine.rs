use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    board::Board,
    r#const::{BLACK, MAX_PLY, WHITE},
    items::{Color, Move, Piece, PieceInfo},
    network::{EvalBuf, HL1, NETWORK},
    search::MAX_HISTORY,
    tt::TranspositionTable,
    uci_print,
};

#[derive(Clone)]
pub struct SharedState {
    pub tt: Arc<TranspositionTable>,
    pub abort: Arc<AtomicBool>,
    pub nodes: Arc<AtomicU64>,
}

impl SharedState {
    pub fn reset(&self) {
        self.tt.clear();
        self.abort.store(false, Ordering::Relaxed);
        self.nodes.store(0, Ordering::Relaxed);
    }
}

pub struct Engine {
    pub board: Board,

    pub history: HistoryTable,
    pub continuation_history: ContinuationHistory,
    pub counter_moves: CountermoveTable,
    pub eval_history: EvalHistory,
    pub capture_history: CaptureHistory,
    pub correction_history: CorrectionHistory,

    pub killers: Killers,

    pub eval_buf: EvalBuf,
    pub accumulators: Accumulators,

    pub thread_id: u16,
    // Shared States across all threads
    pub shared: SharedState,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::start_pos(),

            history: HistoryTable::new(),
            continuation_history: ContinuationHistory::new(),
            counter_moves: CountermoveTable::new(),
            eval_history: EvalHistory::new(),
            capture_history: CaptureHistory::new(),
            correction_history: CorrectionHistory::new(),
            killers: Killers::new(),

            eval_buf: EvalBuf::new(),
            accumulators: Accumulators::new(),

            thread_id: 0,
            shared: SharedState {
                tt: Arc::new(TranspositionTable::new(16)),
                abort: Arc::new(AtomicBool::new(false)),
                nodes: Arc::new(AtomicU64::new(0)),
            },
        }
    }

    pub fn child(&self, thread_id: u16) -> Self {
        Self {
            board: self.board.clone(),

            history: HistoryTable::new(),
            continuation_history: ContinuationHistory::new(),
            counter_moves: CountermoveTable::new(),
            eval_history: EvalHistory::new(),
            capture_history: CaptureHistory::new(),
            correction_history: CorrectionHistory::new(),
            killers: Killers::new(),

            eval_buf: EvalBuf::new(),
            accumulators: Accumulators::new(),

            thread_id: thread_id,
            shared: self.shared.clone(),
        }
    }

    pub fn reset(&mut self) {
        self.board = Board::start_pos();
        self.history = HistoryTable::new();
        self.continuation_history = ContinuationHistory::new();
        self.counter_moves = CountermoveTable::new();
        self.eval_history = EvalHistory::new();
        self.capture_history = CaptureHistory::new();
        self.correction_history = CorrectionHistory::new();
        self.killers = Killers::new();
        self.eval_buf = EvalBuf::new();
        self.accumulators = Accumulators::new();
        self.shared.reset();
    }

    #[inline(always)]
    pub fn info(&self) {
        uci_print!("Roxie v{}\n", env!("CARGO_PKG_VERSION"));
        self.shared.tt.info();
    }
}

pub struct HistoryTable {
    table: [[[i32; 64]; 64]; 2],
}

impl HistoryTable {
    pub fn new() -> Self {
        Self {
            table: [[[0; 64]; 64]; 2],
        }
    }

    #[inline(always)]
    pub fn update(&mut self, stm: usize, from: usize, to: usize, bonus: i32) {
        let h = &mut self.table[stm][from][to];
        *h += bonus - (*h * bonus.abs()) / MAX_HISTORY;
        *h = (*h).clamp(-MAX_HISTORY, MAX_HISTORY);
    }

    #[inline(always)]
    pub fn get(&self, stm: usize, from: usize, to: usize) -> i32 {
        self.table[stm][from][to]
    }
}

pub struct Killers {
    table: [[Move; 2]; MAX_PLY],
}

impl Killers {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            table: [[Move::NULL; 2]; MAX_PLY],
        }
    }

    #[inline(always)]
    pub fn get(&self, ply: i32) -> [Move; 2] {
        self.table[ply as usize]
    }

    #[inline(always)]
    pub fn update(&mut self, mv: Move, ply: i32) {
        let ply = ply as usize;

        if self.table[ply][0] != mv {
            self.table[ply][1] = self.table[ply][0];
            self.table[ply][0] = mv;
        }
    }
}

pub struct CountermoveTable {
    pub table: [[Move; 64]; 64],
}

impl CountermoveTable {
    pub fn new() -> Self {
        Self {
            table: [[Move::NULL; 64]; 64],
        }
    }

    #[inline(always)]
    pub fn update(&mut self, prev_mv: Move, cur_mv: Move) {
        if prev_mv == Move::NULL {
            return;
        }
        self.table[prev_mv.from()][prev_mv.to()] = cur_mv;
    }

    #[inline(always)]
    pub fn get(&self, prev_mv: Move) -> Move {
        self.table[prev_mv.from()][prev_mv.to()]
    }
}

pub struct ContinuationHistory {
    // table[piece_idx * 64 + prev_mv.to][piece_idx * 64 + cur_mv.to]
    pub table: Box<[[i16; 768]; 768]>,
}

impl ContinuationHistory {
    pub fn new() -> Self {
        let table = vec![[0i16; 768]; 768];
        Self {
            table: table.into_boxed_slice().try_into().unwrap(),
        }
    }

    #[inline(always)]
    pub fn get_after_mv(&self, board: &Board, prev_mv: Move, cur_mv: Move) -> i16 {
        if prev_mv == Move::NULL || cur_mv == Move::NULL {
            return 0;
        }

        let prev_idx = Piece::to_idx(board.piece_on(prev_mv.to())) * 64 + prev_mv.to();
        let cur_idx = Piece::to_idx(board.piece_on(cur_mv.to())) * 64 + cur_mv.to();

        self.table[prev_idx][cur_idx]
    }

    #[inline(always)]
    pub fn get(&self, board: &Board, prev_mv: Move, cur_mv: Move) -> i16 {
        debug_assert_ne!(cur_mv, Move::NULL); // cur_mv is expected to be a valid move
        if prev_mv == Move::NULL {
            return 0;
        }

        let prev_idx = Piece::to_idx(board.piece_on(prev_mv.to())) * 64 + prev_mv.to();
        let cur_idx = Piece::to_idx(board.piece_on(cur_mv.from())) * 64 + cur_mv.to();

        self.table[prev_idx][cur_idx]
    }

    #[inline(always)]
    pub fn update(&mut self, board: &Board, prev_mv: Move, cur_mv: Move, bonus: i32) {
        debug_assert_ne!(cur_mv, Move::NULL); // cur_mv is expected to be a valid move
        if prev_mv == Move::NULL {
            return;
        }

        // index = piece_idx * 64 + mv.to
        let prev_idx = Piece::to_idx(board.piece_on(prev_mv.to())) * 64 + prev_mv.to();
        let cur_idx = Piece::to_idx(board.piece_on(cur_mv.from())) * 64 + cur_mv.to();

        let cur_val = self.table[prev_idx][cur_idx];
        let new_val = (cur_val as i32) + bonus - (cur_val as i32 * bonus.abs() / MAX_HISTORY);

        self.table[prev_idx][cur_idx] = new_val.clamp(-MAX_HISTORY, MAX_HISTORY) as i16;
    }
}

pub struct EvalHistory {
    evals: [i16; MAX_PLY],
    checks: [bool; MAX_PLY],
}

impl EvalHistory {
    pub fn new() -> Self {
        Self {
            evals: [0; MAX_PLY],
            checks: [false; MAX_PLY],
        }
    }

    #[inline(always)]
    pub fn update(&mut self, score: i16, in_check: bool, ply: usize) {
        self.evals[ply] = score;
        self.checks[ply] = in_check;
    }

    #[inline(always)]
    pub fn is_improving(&self, current_eval: i16, in_check: bool, ply: usize) -> bool {
        if in_check {
            return false;
        }

        if ply >= 2 {
            let mut past_ply = ply - 2;

            // If we were in check last turn, then check the next previous round, (ply - 4)
            if past_ply >= 2 && self.checks[past_ply] {
                past_ply -= 2;
            }

            return current_eval > self.evals[past_ply];
        }

        true // true for first 2 plies btw
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.evals = [0; MAX_PLY];
        self.checks = [false; MAX_PLY];
    }
}

const MAX_CAP_HISTORY: i32 = 8192;
pub struct CaptureHistory {
    // [atk_piece_idx (0 - 11)][victim_type_idx (0 - 5)][to_sq (0 - 64)]
    table: [[[i32; 64]; 6]; 12],
}

impl CaptureHistory {
    pub fn new() -> Self {
        Self {
            table: [[[0; 64]; 6]; 12],
        }
    }

    #[inline(always)]
    pub fn get(&self, attacker: PieceInfo, victim: PieceInfo, to: usize) -> i32 {
        let att_idx = Piece::to_idx(attacker);
        let vic_type_idx = (Piece::to_idx(victim) % 6) as usize;
        self.table[att_idx][vic_type_idx][to]
    }

    #[inline(always)]
    pub fn update(&mut self, attacker: PieceInfo, victim: PieceInfo, to: usize, bonus: i32) {
        let att_idx = Piece::to_idx(attacker);
        let vic_type_idx = (Piece::to_idx(victim) % 6) as usize;

        let h = &mut self.table[att_idx][vic_type_idx][to];
        *h += bonus - (*h * bonus.abs()) / MAX_CAP_HISTORY;
    }
}

const CORR_HIST_SIZE: usize = 16384; // must be a power of 2
const MAX_CORR_HISTORY: usize = 16384;
pub const CORR_GRAIN: usize = 256; // multiplier for higher precision score while avoiding floats

// Using CORR_GRAIN * 2 halves the table's output, naturally dampening the correction.
const CORR_SCALE: i32 = CORR_GRAIN as i32 * 2;

pub struct CorrectionHistory {
    // [stm color][pawn_key % size]
    table: [[i32; CORR_HIST_SIZE]; 2],
}

impl CorrectionHistory {
    pub fn new() -> Self {
        Self {
            table: [[0; CORR_HIST_SIZE]; 2],
        }
    }

    pub fn get(&self, stm: usize, pawn_key: u64) -> i32 {
        let idx = (pawn_key as usize) & (CORR_HIST_SIZE - 1);
        self.table[stm][idx] / CORR_SCALE
    }

    pub fn update(&mut self, stm: usize, pawn_key: u64, err: i32, depth: i32) {
        let idx = (pawn_key as usize) & (CORR_HIST_SIZE - 1);
        let entry = &mut self.table[stm][idx];

        // using quadratic formula for weight calculation
        let weight = (depth * depth + 2 * depth + 1).min(128);

        // fixed point EMA interpolation blending old value with new error
        let interp = (*entry * (1024 - weight) + err * weight) / 1024;

        *entry = interp.clamp(-(MAX_CORR_HISTORY as i32), MAX_CORR_HISTORY as i32);
    }

    pub fn clear(&mut self) {
        self.table = [[0; CORR_HIST_SIZE]; 2];
    }
}

pub struct Accumulators {
    table: [[[i16; HL1]; 2]; MAX_PLY],
}

impl Accumulators {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            table: [[[0; HL1]; 2]; MAX_PLY],
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, ply: usize) -> &mut [[i16; HL1]; 2] {
        &mut self.table[ply]
    }

    #[inline(always)]
    pub fn get(&self, ply: usize) -> [[i16; HL1]; 2] {
        self.table[ply]
    }

    pub fn setup(&mut self, board: &Board) {
        if let Some(nn) = NETWORK.get() {
            let rebuild = nn.build_acc(board);
            if board.side_to_move() == Color::White {
                self.table[0][WHITE].copy_from_slice(&rebuild[..HL1]);
                self.table[0][BLACK].copy_from_slice(&rebuild[HL1..]);
            } else {
                self.table[0][BLACK].copy_from_slice(&rebuild[..HL1]);
                self.table[0][WHITE].copy_from_slice(&rebuild[HL1..]);
            };
        }
    }
}
