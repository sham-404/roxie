// Aspiration Windows
pub const ASP_DELTA_0: i32 = 37;
pub const ASP_DELTA_1: i32 = 12;
pub const ASP_DELTA_2: i32 = 30;
pub const ASP_DELTA_3: i32 = 45;
pub const ASP_MIN_DEPTH: u16 = 1;
pub const ASP_FACTOR_PCT: f32 = 1.97;

// Internal Iterative Deepening
pub const IID_MIN_DEPTH: u16 = 7;
pub const IID_REDUCTION: u16 = 3;

// Reverse Futility Pruning (RFP)
pub const RFP_DEPTH_IMP: u16 = 2;
pub const RFP_DEPTH_NON: u16 = 2;
pub const RFP_MARGIN_BASE: i16 = 30;
pub const RFP_MARGIN_IMP: i16 = 0;

// Razoring
pub const RAZOR_MAX_DEPTH: u16 = 1;
pub const RAZOR_MARGIN_BASE: i16 = 279;
pub const RAZOR_MARGIN_PER_DEPTH: i16 = 78;

// Null Move Pruning (NMP)
pub const NMP_MIN_DEPTH_IMP: u16 = 1;
pub const NMP_MIN_DEPTH_NON: u16 = 2;
pub const NMP_REDUCE_BASE: u16 = 5;
pub const NMP_REDUCE_DIV: u16 = 5;
pub const NMP_NON_IMP_BONUS: u16 = 0;
pub const NMP_ENDGAME_LIMIT: i32 = 7;

// ProbCut
pub const PC_MIN_DEPTH: u16 = 4;
pub const PC_MARGIN: i32 = 7;
pub const PC_SPLIT_DEPTH: u16 = 9;
pub const PC_REDUCE_SMALL: u16 = 3;
pub const PC_REDUCE_LARGE: u16 = 3;

// Singular Extensions
pub const SING_MIN_DEPTH: u16 = 11;
pub const SING_TT_MARGIN: u16 = 3;
pub const SING_MARGIN_PER_DEPTH: i16 = 2;
pub const SING_DEPTH_DIV: u16 = 2;
pub const SING_DOUBLE_MARGIN: i16 = 32;
pub const SING_EXT_1: i32 = 0;
pub const SING_EXT_2: i32 = 1;

// Late Move Pruning (LMP)
pub const LMP_BASE: usize = 6;
pub const LMP_QUAD_FACTOR: usize = 4;
pub const LMP_NON_IMP_DIV: usize = 3;
pub const LMP_MAX_DEPTH: u16 = 7;

// SEE Pruning
pub const SEE_PRUNE_MAX_DEPTH_IMP: u16 = 2;
pub const SEE_PRUNE_MAX_DEPTH_NON: u16 = 3;
pub const SEE_MARGIN_BASE: i32 = 87;
pub const SEE_MARGIN_IMP: i32 = 14;

// Futility Pruning
pub const FUT_MAX_DEPTH_IMP: u16 = 4;
pub const FUT_MAX_DEPTH_NON: u16 = 7;
pub const FUT_MARGIN_BASE: i16 = 73;
pub const FUT_MARGIN_IMP: i16 = 1;

// Late Move Reductions (LMR)
pub const LMR_BASE_PCT: f64 = 1.33;
pub const LMR_DIV_PCT: f64 = 2.47;
pub const LMR_MIN_DEPTH_IMP: u16 = 2;
pub const LMR_MIN_DEPTH_NON: u16 = 2;
pub const LMR_MIN_QUIET: usize = 1;
pub const LMR_HIST_FACTOR: i32 = 8192;
pub const LMR_HIST_CLAMP: i32 = 5;

pub const LMR_CONT_HIST_SHIFT: i32 = 2;
pub const LMR_NON_IMP_BONUS: i32 = 0;
pub const LMR_COUNTER_KILLER_BONUS: i32 = 4;

// History and Extensions
pub const CHECK_EXT_MAX: u8 = 1;
pub const HIST_BONUS_QUAD_FACTOR: i32 = 2;
pub const HIST_BONUS_MAX: i32 = 389;

// Quiescence Search
pub const Q_EP_GAIN: i16 = 75;
pub const Q_DELTA_MARGIN: i16 = 154;

// Piece Values (SEE)
pub const SEE_PAWN: i32 = 97;
pub const SEE_KNIGHT: i32 = 311;
pub const SEE_BISHOP: i32 = 323;
pub const SEE_ROOK: i32 = 463;
pub const SEE_QUEEN: i32 = 929;

// History tables
pub const HIST_SHIFT: i32 = 2;
pub const CONT_HIST_SHIFT_1: i32 = 3;
pub const CONT_HIST_SHIFT_2: i32 = 2;
pub const CONT_HIST_SHIFT_4: i32 = 1;

