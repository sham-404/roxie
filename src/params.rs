// Working of params.rs:
//
// The parameters needed for tuning is declared as a bunch inside the macro
// define_tunables!. This macro pattern matches and feed one parameter at a
// time (separated by ';') to declare! macro. declare! declared the parameter
// as const, if it is a normal build, and declares it as static Atomic data type
// if it is a spsa tunable build. and in define_tunables!, we have spin_options()
// function, which is generated only on spsa build, which bundles all the
// tunable parameters into a vector, so that it is easy for exposing the
// parameters to uci as well as easy for spsa tuning (as it is used to
// check whether the spsa option is present)
//
// Each parameter is read through param! macro. It reads directly as a normal
// variable in a normal build, and uses atomic load to read on spsa build.
// We use TunableRead trait for atomic spsa reading, because there are different
// types of Atomics used (for eg, f64 is converted to AtomicU64 which needs
// special loading) for each parameters.

/// used to get the constant parameters
///
/// Constant parameters are exposed as `pub const` in normal
/// builds and `pub static <name>: Atomic` in spsa feature
/// build. due to this dynamic nature, param! is used to retrieve
/// the constants effectively
#[macro_export]
macro_rules! param {
    ($name:ident) => {{
        #[cfg(not(feature = "spsa"))]
        {
            $crate::params::$name
        }
        #[cfg(feature = "spsa")]
        {
            $crate::params::TunableRead::read(&$crate::params::$name)
        }
    }};
}

/// A structure to hold the tunable parameters (in spsa build)
///
/// Float parameters are exposed as integers scaled by 100.
#[cfg(feature = "spsa")]
pub struct SpinOption {
    name: &'static str,
    default: i64,
    min: i64,
    max: i64,
    set: fn(i64),
}

#[cfg(feature = "spsa")]
impl SpinOption {
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The original compile-time default, not the current (possibly tuned)
    /// value, so repeated uci commands always report the true default
    pub fn default(&self) -> i64 {
        self.default
    }

    pub fn min(&self) -> i64 {
        self.min
    }

    pub fn max(&self) -> i64 {
        self.max
    }

    /// Update the parameter, clamping to the [min, max] range
    pub fn set(&self, value: i64) {
        (self.set)(value.clamp(self.min, self.max));
    }
}

/// Loading a tunable atomic.
#[cfg(feature = "spsa")]
pub trait TunableRead {
    type Output;

    fn read(&self) -> Self::Output;
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicI16 {
    type Output = i16;

    #[inline(always)]
    fn read(&self) -> i16 {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicU16 {
    type Output = u16;

    #[inline(always)]
    fn read(&self) -> u16 {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicU8 {
    type Output = u8;

    #[inline(always)]
    fn read(&self) -> u8 {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicI32 {
    type Output = i32;

    #[inline(always)]
    fn read(&self) -> i32 {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicUsize {
    type Output = usize;

    #[inline(always)]
    fn read(&self) -> usize {
        self.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicU32 {
    type Output = f32;

    #[inline(always)]
    fn read(&self) -> f32 {
        f32::from_bits(self.load(std::sync::atomic::Ordering::Relaxed))
    }
}

#[cfg(feature = "spsa")]
impl TunableRead for std::sync::atomic::AtomicU64 {
    type Output = f64;

    #[inline(always)]
    fn read(&self) -> f64 {
        f64::from_bits(self.load(std::sync::atomic::Ordering::Relaxed))
    }
}

/// Defines the actual declaration part of parameters:
/// pub const for normal builds and pub static atomics for spsa builds
macro_rules! declare {
    ($name:ident : i16 = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: i16 = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicI16 =
            std::sync::atomic::AtomicI16::new($default);
    };
    ($name:ident : u16 = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: u16 = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicU16 =
            std::sync::atomic::AtomicU16::new($default);
    };
    ($name:ident : u8 = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: u8 = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new($default);
    };
    ($name:ident : i32 = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: i32 = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicI32 =
            std::sync::atomic::AtomicI32::new($default);
    };
    ($name:ident : usize = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: usize = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new($default);
    };
    ($name:ident : f32 = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: f32 = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(($default as f32).to_bits());
    };
    ($name:ident : f64 = $default:expr, $min:expr, $max:expr) => {
        #[cfg(not(feature = "spsa"))]
        pub const $name: f64 = $default;
        #[cfg(feature = "spsa")]
        pub static $name: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(($default as f64).to_bits());
    };
}

// Building SpinOption for uci parameter exposing
#[cfg(feature = "spsa")]
macro_rules! spin_option {
    ($name:ident : i16 = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: $default as i64,
            min: $min as i64,
            max: $max as i64,
            set: |value: i64| $name.store(value as i16, std::sync::atomic::Ordering::Relaxed),
        }
    };
    ($name:ident : u16 = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: $default as i64,
            min: $min as i64,
            max: $max as i64,
            set: |value: i64| $name.store(value as u16, std::sync::atomic::Ordering::Relaxed),
        }
    };
    ($name:ident : u8 = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: $default as i64,
            min: $min as i64,
            max: $max as i64,
            set: |value: i64| $name.store(value as u8, std::sync::atomic::Ordering::Relaxed),
        }
    };
    ($name:ident : i32 = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: $default as i64,
            min: $min as i64,
            max: $max as i64,
            set: |value: i64| $name.store(value as i32, std::sync::atomic::Ordering::Relaxed),
        }
    };
    ($name:ident : usize = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: $default as i64,
            min: $min as i64,
            max: $max as i64,
            set: |value: i64| $name.store(value as usize, std::sync::atomic::Ordering::Relaxed),
        }
    };
    ($name:ident : f32 = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: (($default as f64) * 100.0).round() as i64,
            min: (($min as f64) * 100.0).round() as i64,
            max: (($max as f64) * 100.0).round() as i64,
            set: |value: i64| {
                $name.store(
                    ((value as f32) / 100.0).to_bits(),
                    std::sync::atomic::Ordering::Relaxed,
                )
            },
        }
    };
    ($name:ident : f64 = $default:expr, $min:expr, $max:expr) => {
        SpinOption {
            name: stringify!($name),
            default: (($default as f64) * 100.0).round() as i64,
            min: (($min as f64) * 100.0).round() as i64,
            max: (($max as f64) * 100.0).round() as i64,
            set: |value: i64| {
                $name.store(
                    ((value as f64) / 100.0).to_bits(),
                    std::sync::atomic::Ordering::Relaxed,
                )
            },
        }
    };
}

/// Place for defining all the spsa tunable constants
macro_rules! define_tunables {
    ($($name:ident : $ty:tt = $default:expr, $min:expr, $max:expr;)*) => {
        $(
            declare!($name : $ty = $default, $min, $max);
        )*

        #[cfg(feature = "spsa")]
        pub fn spin_options() -> Vec<SpinOption> {
            vec![
                $(spin_option!($name : $ty = $default, $min, $max),)*
            ]
        }
    };
}

define_tunables! {
    // Aspiration Windows
    ASP_DELTA_0: i32 = 37, 1, 200;
    ASP_DELTA_1: i32 = 12, 1, 200;
    ASP_DELTA_2: i32 = 30, 1, 200;
    ASP_DELTA_3: i32 = 45, 1, 200;
    ASP_MIN_DEPTH: u16 = 1, 0, 12;
    ASP_FACTOR_PCT: f32 = 1.97, 1.0, 4.0;

    // Internal Iterative Deepening
    IID_MIN_DEPTH: u16 = 7, 1, 20;
    IID_REDUCTION: u16 = 3, 0, 10;

    // Reverse Futility Pruning (RFP)
    RFP_DEPTH_IMP: u16 = 2, 1, 10;
    RFP_DEPTH_NON: u16 = 2, 1, 10;
    RFP_MARGIN_BASE: i16 = 30, 0, 300;
    RFP_MARGIN_IMP: i16 = 0, -100, 100;

    // Razoring
    RAZOR_MAX_DEPTH: u16 = 1, 0, 10;
    RAZOR_MARGIN_BASE: i16 = 279, 0, 1000;
    RAZOR_MARGIN_PER_DEPTH: i16 = 78, 0, 500;

    // Null Move Pruning (NMP)
    NMP_MIN_DEPTH_IMP: u16 = 1, 0, 10;
    NMP_MIN_DEPTH_NON: u16 = 2, 0, 10;
    NMP_REDUCE_BASE: u16 = 5, 0, 15;
    NMP_REDUCE_DIV: u16 = 5, 1, 20;
    NMP_NON_IMP_BONUS: u16 = 0, 0, 10;
    NMP_ENDGAME_LIMIT: i32 = 7, 0, 24;

    // ProbCut
    PC_MIN_DEPTH: u16 = 4, 1, 15;
    PC_MARGIN: i32 = 7, 0, 500;
    PC_SPLIT_DEPTH: u16 = 9, 1, 30;
    PC_REDUCE_SMALL: u16 = 3, 0, 10;
    PC_REDUCE_LARGE: u16 = 3, 0, 10;

    // Singular Extensions
    SING_MIN_DEPTH: u16 = 11, 1, 30;
    SING_TT_MARGIN: u16 = 3, 0, 10;
    SING_MARGIN_PER_DEPTH: i16 = 2, 0, 20;
    SING_DEPTH_DIV: u16 = 2, 1, 10;
    SING_DOUBLE_MARGIN: i16 = 32, 0, 200;
    SING_EXT_1: i32 = 0, 0, 10;
    SING_EXT_2: i32 = 1, 0, 10;

    // Late Move Pruning (LMP)
    LMP_BASE: usize = 6, 0, 100;
    LMP_QUAD_FACTOR: usize = 4, 0, 50;
    LMP_NON_IMP_DIV: usize = 3, 1, 20;
    LMP_MAX_DEPTH: u16 = 7, 1, 30;

    // SEE Pruning
    SEE_PRUNE_MAX_DEPTH_IMP: u16 = 2, 0, 10;
    SEE_PRUNE_MAX_DEPTH_NON: u16 = 3, 0, 10;
    SEE_MARGIN_BASE: i32 = 87, 0, 1000;
    SEE_MARGIN_IMP: i32 = 14, 0, 500;

    // Futility Pruning
    FUT_MAX_DEPTH_IMP: u16 = 4, 0, 20;
    FUT_MAX_DEPTH_NON: u16 = 7, 0, 20;
    FUT_MARGIN_BASE: i16 = 73, 0, 500;
    FUT_MARGIN_IMP: i16 = 1, 0, 200;

    // Late Move Reductions (LMR)
    LMR_BASE_PCT: f64 = 1.33, 0.1, 5.0;
    LMR_DIV_PCT: f64 = 2.47, 0.1, 10.0;
    LMR_MIN_DEPTH_IMP: u16 = 2, 0, 10;
    LMR_MIN_DEPTH_NON: u16 = 2, 0, 10;
    LMR_MIN_QUIET: usize = 1, 0, 20;
    LMR_HIST_FACTOR: i32 = 8192, 1, 32768;
    LMR_HIST_CLAMP: i32 = 5, 0, 20;
    LMR_CONT_HIST_SHIFT: i32 = 2, 0, 8;
    LMR_NON_IMP_BONUS: i32 = 0, 0, 20;
    LMR_COUNTER_KILLER_BONUS: i32 = 4, 0, 20;

    // History and Extensions
    CHECK_EXT_MAX: u8 = 1, 0, 4;
    HIST_BONUS_QUAD_FACTOR: i32 = 2, 0, 20;
    HIST_BONUS_MAX: i32 = 389, 0, 2000;

    // Quiescence Search
    Q_EP_GAIN: i16 = 75, 0, 500;

    Q_DELTA_MARGIN: i16 = 154, 0, 500;
    // Piece Values (SEE)
    SEE_PAWN: i32 = 97, 0, 1000;
    SEE_KNIGHT: i32 = 311, 0, 2000;
    SEE_BISHOP: i32 = 323, 0, 2000;
    SEE_ROOK: i32 = 463, 0, 2000;
    SEE_QUEEN: i32 = 929, 0, 4000;

    // History tables
    HIST_SHIFT: i32 = 2, 0, 8;
    CONT_HIST_SHIFT_1: i32 = 3, 0, 8;
    CONT_HIST_SHIFT_2: i32 = 2, 0, 8;
    CONT_HIST_SHIFT_4: i32 = 1, 0, 8;

    // Correction history table
    CORR_WEIGHT_PAWN: i32 = 1, 0, 32;
    CORR_WEIGHT_MINOR: i32 = 1, 0, 32;
    CORR_WEIGHT_MAJOR: i32 = 1, 0, 32;
    CORR_WEIGHT_NON_PAWN: i32 = 1, 0, 32;
    CORR_WEIGHT_MATERIAL: i32 = 1, 0, 32;
    CORR_WEIGHT_KING: i32 = 1, 0, 32;
}
