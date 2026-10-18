use crate::board::Board;
use crate::r#const::MAX_PLY;
use crate::items::{Color, Move, MoveFlag, Piece};
use crate::search::MATE;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, RwLock};

pub const WDL: usize = 0;
pub const DTM: usize = 1;
pub const DTZ: usize = 2;

pub const PIECE_ENC: usize = 0;
pub const FILE_ENC: usize = 1;
pub const RANK_ENC: usize = 2;

pub const TB_PIECES: usize = 7;
pub const TB_HASHBITS: usize = 12;
pub static PROBE_MAX_PIECES: AtomicUsize = AtomicUsize::new(0);

#[rustfmt::skip]
pub const OFF_DIAG: [i8; 64] = [
     0, -1, -1, -1, -1, -1, -1, -1,
     1,  0, -1, -1, -1, -1, -1, -1,
     1,  1,  0, -1, -1, -1, -1, -1,
     1,  1,  1,  0, -1, -1, -1, -1,
     1,  1,  1,  1,  0, -1, -1, -1,
     1,  1,  1,  1,  1,  0, -1, -1,
     1,  1,  1,  1,  1,  1,  0, -1,
     1,  1,  1,  1,  1,  1,  1,  0,
];

#[rustfmt::skip]
pub const TRIANGLE: [u8; 64] = [
     6,  0,  1,  2,  2,  1,  0,  6,
     0,  7,  3,  4,  4,  3,  7,  0,
     1,  3,  8,  5,  5,  8,  3,  1,
     2,  4,  5,  9,  9,  5,  4,  2,
     2,  4,  5,  9,  9,  5,  4,  2,
     1,  3,  8,  5,  5,  8,  3,  1,
     0,  7,  3,  4,  4,  3,  7,  0,
     6,  0,  1,  2,  2,  1,  0,  6,
];

#[rustfmt::skip]
pub const FLIP_DIAG: [u8; 64] = [
     0,  8, 16, 24, 32, 40, 48, 56,
     1,  9, 17, 25, 33, 41, 49, 57,
     2, 10, 18, 26, 34, 42, 50, 58,
     3, 11, 19, 27, 35, 43, 51, 59,
     4, 12, 20, 28, 36, 44, 52, 60,
     5, 13, 21, 29, 37, 45, 53, 61,
     6, 14, 22, 30, 38, 46, 54, 62,
     7, 15, 23, 31, 39, 47, 55, 63,
];

#[rustfmt::skip]
pub const LOWER: [u8; 64] = [
    28,  0,  1,  2,  3,  4,  5,  6,
     0, 29,  7,  8,  9, 10, 11, 12,
     1,  7, 30, 13, 14, 15, 16, 17,
     2,  8, 13, 31, 18, 19, 20, 21,
     3,  9, 14, 18, 32, 22, 23, 24,
     4, 10, 15, 19, 22, 33, 25, 26,
     5, 11, 16, 20, 23, 25, 34, 27,
     6, 12, 17, 21, 24, 26, 27, 35,
];

#[rustfmt::skip]
pub const DIAG: [u8; 64] = [
     0,  0,  0,  0,  0,  0,  0,  8,
     0,  1,  0,  0,  0,  0,  9,  0,
     0,  0,  2,  0,  0, 10,  0,  0,
     0,  0,  0,  3, 11,  0,  0,  0,
     0,  0,  0, 12,  4,  0,  0,  0,
     0,  0, 13,  0,  0,  5,  0,  0,
     0, 14,  0,  0,  0,  0,  6,  0,
    15,  0,  0,  0,  0,  0,  0,  7,
];

#[rustfmt::skip]
pub const FLAP: [[u8; 64]; 2] = [
    [
         0,  0,  0,  0,  0,  0,  0,  0,
         0,  6, 12, 18, 18, 12,  6,  0,
         1,  7, 13, 19, 19, 13,  7,  1,
         2,  8, 14, 20, 20, 14,  8,  2,
         3,  9, 15, 21, 21, 15,  9,  3,
         4, 10, 16, 22, 22, 16, 10,  4,
         5, 11, 17, 23, 23, 17, 11,  5,
         0,  0,  0,  0,  0,  0,  0,  0,
    ],
    [
         0,  0,  0,  0,  0,  0,  0,  0,
         0,  1,  2,  3,  3,  2,  1,  0,
         4,  5,  6,  7,  7,  6,  5,  4,
         8,  9, 10, 11, 11, 10,  9,  8,
        12, 13, 14, 15, 15, 14, 13, 12,
        16, 17, 18, 19, 19, 18, 17, 16,
        20, 21, 22, 23, 23, 22, 21, 20,
         0,  0,  0,  0,  0,  0,  0,  0,
    ]
];

#[rustfmt::skip]
pub const PAWN_TWIST: [[u8; 64]; 2] = [
    [
         0,  0,  0,  0,  0,  0,  0,  0,
        47, 35, 23, 11, 10, 22, 34, 46,
        45, 33, 21,  9,  8, 20, 32, 44,
        43, 31, 19,  7,  6, 18, 30, 42,
        41, 29, 17,  5,  4, 16, 28, 40,
        39, 27, 15,  3,  2, 14, 26, 38,
        37, 25, 13,  1,  0, 12, 24, 36,
         0,  0,  0,  0,  0,  0,  0,  0,
    ],
    [
         0,  0,  0,  0,  0,  0,  0,  0,
        47, 45, 43, 41, 40, 42, 44, 46,
        39, 37, 35, 33, 32, 34, 36, 38,
        31, 29, 27, 25, 24, 26, 28, 30,
        23, 21, 19, 17, 16, 18, 20, 22,
        15, 13, 11,  9,  8, 10, 12, 14,
         7,  5,  3,  1,  0,  2,  4,  6,
         0,  0,  0,  0,  0,  0,  0,  0,
    ]
];

#[rustfmt::skip]
pub const KK_IDX: [[i16; 64]; 10] = [
    [
          -1,   -1,   -1,    0,    1,    2,    3,    4,
          -1,   -1,   -1,    5,    6,    7,    8,    9,
          10,   11,   12,   13,   14,   15,   16,   17,
          18,   19,   20,   21,   22,   23,   24,   25,
          26,   27,   28,   29,   30,   31,   32,   33,
          34,   35,   36,   37,   38,   39,   40,   41,
          42,   43,   44,   45,   46,   47,   48,   49,
          50,   51,   52,   53,   54,   55,   56,   57,
    ],
    [
          58,   -1,   -1,   -1,   59,   60,   61,   62,
          63,   -1,   -1,   -1,   64,   65,   66,   67,
          68,   69,   70,   71,   72,   73,   74,   75,
          76,   77,   78,   79,   80,   81,   82,   83,
          84,   85,   86,   87,   88,   89,   90,   91,
          92,   93,   94,   95,   96,   97,   98,   99,
         100,  101,  102,  103,  104,  105,  106,  107,
         108,  109,  110,  111,  112,  113,  114,  115,
    ],
    [
         116,  117,   -1,   -1,   -1,  118,  119,  120,
         121,  122,   -1,   -1,   -1,  123,  124,  125,
         126,  127,  128,  129,  130,  131,  132,  133,
         134,  135,  136,  137,  138,  139,  140,  141,
         142,  143,  144,  145,  146,  147,  148,  149,
         150,  151,  152,  153,  154,  155,  156,  157,
         158,  159,  160,  161,  162,  163,  164,  165,
         166,  167,  168,  169,  170,  171,  172,  173,
    ],
    [
         174,   -1,   -1,   -1,  175,  176,  177,  178,
         179,   -1,   -1,   -1,  180,  181,  182,  183,
         184,   -1,   -1,   -1,  185,  186,  187,  188,
         189,  190,  191,  192,  193,  194,  195,  196,
         197,  198,  199,  200,  201,  202,  203,  204,
         205,  206,  207,  208,  209,  210,  211,  212,
         213,  214,  215,  216,  217,  218,  219,  220,
         221,  222,  223,  224,  225,  226,  227,  228,
    ],
    [
         229,  230,   -1,   -1,   -1,  231,  232,  233,
         234,  235,   -1,   -1,   -1,  236,  237,  238,
         239,  240,   -1,   -1,   -1,  241,  242,  243,
         244,  245,  246,  247,  248,  249,  250,  251,
         252,  253,  254,  255,  256,  257,  258,  259,
         260,  261,  262,  263,  264,  265,  266,  267,
         268,  269,  270,  271,  272,  273,  274,  275,
         276,  277,  278,  279,  280,  281,  282,  283,
    ],
    [
         284,  285,  286,  287,  288,  289,  290,  291,
         292,  293,   -1,   -1,   -1,  294,  295,  296,
         297,  298,   -1,   -1,   -1,  299,  300,  301,
         302,  303,   -1,   -1,   -1,  304,  305,  306,
         307,  308,  309,  310,  311,  312,  313,  314,
         315,  316,  317,  318,  319,  320,  321,  322,
         323,  324,  325,  326,  327,  328,  329,  330,
         331,  332,  333,  334,  335,  336,  337,  338,
    ],
    [
          -1,   -1,  339,  340,  341,  342,  343,  344,
          -1,   -1,  345,  346,  347,  348,  349,  350,
          -1,   -1,  441,  351,  352,  353,  354,  355,
          -1,   -1,   -1,  442,  356,  357,  358,  359,
          -1,   -1,   -1,   -1,  443,  360,  361,  362,
          -1,   -1,   -1,   -1,   -1,  444,  363,  364,
          -1,   -1,   -1,   -1,   -1,   -1,  445,  365,
          -1,   -1,   -1,   -1,   -1,   -1,   -1,  446,
    ],
    [
          -1,   -1,   -1,  366,  367,  368,  369,  370,
          -1,   -1,   -1,  371,  372,  373,  374,  375,
          -1,   -1,   -1,  376,  377,  378,  379,  380,
          -1,   -1,   -1,  447,  381,  382,  383,  384,
          -1,   -1,   -1,   -1,  448,  385,  386,  387,
          -1,   -1,   -1,   -1,   -1,  449,  388,  389,
          -1,   -1,   -1,   -1,   -1,   -1,  450,  390,
          -1,   -1,   -1,   -1,   -1,   -1,   -1,  451,
    ],
    [
         452,  391,  392,  393,  394,  395,  396,  397,
          -1,   -1,   -1,   -1,  398,  399,  400,  401,
          -1,   -1,   -1,   -1,  402,  403,  404,  405,
          -1,   -1,   -1,   -1,  406,  407,  408,  409,
          -1,   -1,   -1,   -1,  453,  410,  411,  412,
          -1,   -1,   -1,   -1,   -1,  454,  413,  414,
          -1,   -1,   -1,   -1,   -1,   -1,  455,  415,
          -1,   -1,   -1,   -1,   -1,   -1,   -1,  456,
    ],
    [
         457,  416,  417,  418,  419,  420,  421,  422,
          -1,  458,  423,  424,  425,  426,  427,  428,
          -1,   -1,   -1,   -1,   -1,  429,  430,  431,
          -1,   -1,   -1,   -1,   -1,  432,  433,  434,
          -1,   -1,   -1,   -1,   -1,  435,  436,  437,
          -1,   -1,   -1,   -1,   -1,  459,  438,  439,
          -1,   -1,   -1,   -1,   -1,   -1,  460,  440,
          -1,   -1,   -1,   -1,   -1,   -1,   -1,  461,
    ],
];

pub const FILE_TO_FILE: [u8; 8] = [0, 1, 2, 3, 3, 2, 1, 0];
pub const WDL_TO_MAP: [usize; 5] = [1, 3, 0, 2, 0];
pub const PA_FLAGS: [u8; 5] = [8, 0, 0, 0, 4];
pub const WDL_TO_DTZ: [i32; 5] = [-1, -101, 0, 101, 1];

pub struct TbIndices {
    pub binomial: [[usize; 64]; 65],
    pub pawn_idx: [[[usize; 24]; 6]; 2],
    pub pawn_factor_file: [[usize; 4]; 6],
    pub pawn_factor_rank: [[usize; 6]; 6],
}

pub static INDICES: LazyLock<TbIndices> = LazyLock::new(|| {
    let mut binomial = [[0usize; 64]; 65];
    let mut pawn_idx = [[[0usize; 24]; 6]; 2];
    let mut pawn_factor_file = [[0usize; 4]; 6];
    let mut pawn_factor_rank = [[0usize; 6]; 6];

    for i in 0..65 {
        for j in 0..64 {
            let mut f: usize = 1;
            let mut l: usize = 1;
            for k in 0..i {
                if j < k {
                    f = 0;
                    break;
                }
                f = f.wrapping_mul(j - k);
                l = l.wrapping_mul(k + 1);
            }
            binomial[i][j] = if l > 0 { f / l } else { 0 };
        }
    }

    for i in 0..6 {
        let mut s = 0;
        for j in 0..24 {
            pawn_idx[0][i][j] = s;
            s += binomial[i][PAWN_TWIST[0][(1 + (j % 6)) * 8 + (j / 6)] as usize];
            if (j + 1) % 6 == 0 {
                pawn_factor_file[i][j / 6] = s;
                s = 0;
            }
        }
    }

    for i in 0..6 {
        let mut s = 0;
        for j in 0..24 {
            pawn_idx[1][i][j] = s;
            s += binomial[i][PAWN_TWIST[1][(1 + (j / 4)) * 8 + (j % 4)] as usize];
            if (j + 1) % 4 == 0 {
                pawn_factor_rank[i][j / 4] = s;
                s = 0;
            }
        }
    }

    TbIndices {
        binomial,
        pawn_idx,
        pawn_factor_file,
        pawn_factor_rank,
    }
});

#[cfg(unix)]
mod os_mmap {
    use std::os::unix::io::AsRawFd;

    unsafe extern "C" {
        fn mmap(
            addr: *mut u8,
            length: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: isize,
        ) -> *mut u8;
        fn munmap(addr: *mut u8, length: usize) -> i32;
    }

    const PROT_READ: i32 = 1;
    const MAP_SHARED: i32 = 1;
    const MAP_FAILED: *mut u8 = !0 as *mut u8;

    pub struct TbData {
        ptr: *const u8,
        len: usize,
    }

    unsafe impl Send for TbData {}
    unsafe impl Sync for TbData {}

    impl TbData {
        pub fn new(path: &std::path::Path) -> std::io::Result<Self> {
            let file = std::fs::File::open(path)?;
            let len = file.metadata()?.len() as usize;
            let fd = file.as_raw_fd();

            let ptr = unsafe { mmap(std::ptr::null_mut(), len, PROT_READ, MAP_SHARED, fd, 0) };

            if ptr == MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self { ptr, len })
        }

        pub fn ptr(&self) -> *const u8 {
            self.ptr
        }

        pub fn len(&self) -> usize {
            self.len
        }
    }

    impl Drop for TbData {
        fn drop(&mut self) {
            unsafe {
                munmap(self.ptr as *mut u8, self.len);
            }
        }
    }
}

#[cfg(windows)]
mod os_mmap {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    type HANDLE = *mut c_void;
    type DWORD = u32;
    type SIZE_T = usize;

    extern "system" {
        fn CreateFileMappingW(
            hFile: HANDLE,
            lpFileMappingAttributes: *mut c_void,
            flProtect: DWORD,
            dwMaximumSizeHigh: DWORD,
            dwMaximumSizeLow: DWORD,
            lpName: *const u16,
        ) -> HANDLE;

        fn MapViewOfFile(
            hFileMappingObject: HANDLE,
            dwDesiredAccess: DWORD,
            dwFileOffsetHigh: DWORD,
            dwFileOffsetLow: DWORD,
            dwNumberOfBytesToMap: SIZE_T,
        ) -> *mut c_void;

        fn UnmapViewOfFile(lpBaseAddress: *const c_void) -> i32;
        fn CloseHandle(hObject: HANDLE) -> i32;
    }

    const PAGE_READONLY: DWORD = 0x02;
    const FILE_MAP_READ: DWORD = 4;

    pub struct TbData {
        ptr: *const u8,
        len: usize,
        map_handle: HANDLE,
    }

    unsafe impl Send for TbData {}
    unsafe impl Sync for TbData {}

    impl TbData {
        pub fn new(path: &std::path::Path) -> std::io::Result<Self> {
            let file = std::fs::File::open(path)?;
            let len = file.metadata()?.len() as usize;
            let handle = file.as_raw_handle() as HANDLE;

            let map_handle = unsafe {
                CreateFileMappingW(
                    handle,
                    std::ptr::null_mut(),
                    PAGE_READONLY,
                    0,
                    0,
                    std::ptr::null(),
                )
            };

            if map_handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr = unsafe { MapViewOfFile(map_handle, FILE_MAP_READ, 0, 0, 0) as *mut u8 };

            if ptr.is_null() {
                unsafe {
                    CloseHandle(map_handle);
                }
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self {
                ptr,
                len,
                map_handle,
            })
        }

        pub fn ptr(&self) -> *const u8 {
            self.ptr
        }

        pub fn len(&self) -> usize {
            self.len
        }
    }

    impl Drop for TbData {
        fn drop(&mut self) {
            unsafe {
                UnmapViewOfFile(self.ptr as *const c_void);
                CloseHandle(self.map_handle);
            }
        }
    }
}

pub use os_mmap::TbData;

#[derive(Default)]
pub struct PairsData {
    pub index_table: *const u8,
    pub size_table: *const u16,
    pub data: *const u8,
    pub offset: *const u16,
    pub sym_pat: *const u8,
    pub sym_len: Vec<u8>,
    pub base: Vec<u64>,
    pub block_size: u8,
    pub idx_bits: u8,
    pub min_len: u8,
    pub const_value: [u8; 2],
}

unsafe impl Send for PairsData {}
unsafe impl Sync for PairsData {}

#[derive(Default)]
pub struct EncInfo {
    pub precomp: Option<Box<PairsData>>,
    pub factor: [usize; TB_PIECES],
    pub pieces: [u8; TB_PIECES],
    pub norm: [u8; TB_PIECES],
}

#[derive(Default)]
pub struct BaseEntry {
    pub key: u64,
    pub sym_key: u64,
    pub mmap: [Option<TbData>; 3],
    pub num: u8,
    pub symmetric: bool,
    pub has_pawns: bool,
    pub has_dtm: bool,
    pub has_dtz: bool,
    pub kk_enc: bool,
    pub pawns: [u8; 2],
    pub dtm_loss_only: bool,
    pub ei: Vec<EncInfo>,
    pub ei_dtz: Vec<EncInfo>,
    pub ready: bool,

    // DTZ Table Metadata - use usize to allow implicit Send/Sync without boxing lifetime woes
    pub dtz_map: usize,
    pub piece_dtz_map_idx: [u16; 4],
    pub pawn_dtz_map_idx: [[u16; 4]; 4],
    pub piece_dtz_flags: u8,
    pub pawn_dtz_flags: [u8; 4],
}

#[derive(Clone, Default)]
pub struct TbRootMove {
    pub mv: Move,
    pub pv: Vec<Move>,
    pub tb_rank: i32,
    pub tb_score: i32,
}

pub struct TbRootMoves {
    pub moves: Vec<TbRootMove>,
}

pub static TB_PATHS: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(Vec::new()));
pub static TB_CACHE: LazyLock<RwLock<HashMap<u64, Option<Arc<BaseEntry>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn init_tb(path_str: &str) {
    let mut paths = TB_PATHS.write().unwrap();
    paths.clear();
    TB_CACHE.write().unwrap().clear();

    if path_str.is_empty() || path_str == "<empty>" {
        PROBE_MAX_PIECES.store(0, Ordering::Relaxed);
        return;
    }

    let separator = if cfg!(windows) { ';' } else { ':' };
    let mut max_found_pieces = 0;

    for dir_path in path_str
        .split(separator)
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        paths.push(dir_path.to_string());

        let Ok(entries) = std::fs::read_dir(dir_path) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rtbw") {
                continue;
            }

            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let pieces = stem.chars().filter(|c| "KQRBNP".contains(*c)).count();
                max_found_pieces = max_found_pieces.max(pieces);
            }
        }
    }

    PROBE_MAX_PIECES.store(max_found_pieces, Ordering::Relaxed);
}

pub fn find_tb_file(filename: &str) -> Option<PathBuf> {
    let paths = TB_PATHS.read().unwrap();
    for p in paths.iter() {
        let full_path = std::path::Path::new(p).join(filename);
        if full_path.exists() {
            return Some(full_path);
        }
    }
    None
}

fn subfactor(k: usize, n: usize) -> usize {
    let mut f = n;
    let mut l = 1;
    for i in 1..k {
        f *= n - i;
        l *= i + 1;
    }
    f / l
}

pub fn get_tb_filename(board: &Board, flip: bool) -> String {
    let c1 = if flip { Piece::BLACK } else { Piece::WHITE };
    let c2 = if flip { Piece::WHITE } else { Piece::BLACK };

    let pieces = [
        Piece::KING,
        Piece::QUEEN,
        Piece::ROOK,
        Piece::BISHOP,
        Piece::KNIGHT,
        Piece::PAWN,
    ];
    let chars = ['K', 'Q', 'R', 'B', 'N', 'P'];

    let mut s = String::new();
    for (i, &pt) in pieces.iter().enumerate() {
        let count = board.bb(c1 | pt).count_ones();
        for _ in 0..count {
            s.push(chars[i]);
        }
    }
    s.push('v');
    for (i, &pt) in pieces.iter().enumerate() {
        let count = board.bb(c2 | pt).count_ones();
        for _ in 0..count {
            s.push(chars[i]);
        }
    }
    s
}

unsafe fn init_enc_info(
    ei: &mut EncInfo,
    pawns: [u8; 2],
    num: u8,
    kk_enc: bool,
    data: *const u8,
    shift: u8,
    t: usize,
    enc: usize,
) -> usize {
    let more_pawns = enc != PIECE_ENC && pawns[1] > 0;
    let offset = if more_pawns { 2 } else { 1 };

    unsafe {
        for i in 0..(num as usize) {
            ei.pieces[i] = (*data.add(i + offset) >> shift) & 0x0f;
            ei.norm[i] = 0;
        }

        let order = (*data >> shift) & 0x0f;
        let order2 = if more_pawns {
            (*data.add(1) >> shift) & 0x0f
        } else {
            0x0f
        };

        let mut k = if enc != PIECE_ENC {
            pawns[0] as usize
        } else if kk_enc {
            2
        } else {
            3
        };
        ei.norm[0] = k as u8;

        if more_pawns {
            ei.norm[k] = pawns[1];
            k += pawns[1] as usize;
        }

        let mut i = k;
        let num_usize = num as usize;
        while i < num_usize {
            let mut j = i;
            while j < num_usize && ei.pieces[j] == ei.pieces[i] {
                ei.norm[i] += 1;
                j += 1;
            }
            i += ei.norm[i] as usize;
        }

        let mut n = 64 - k;
        let mut f: usize = 1;

        let mut idx = 0;
        while k < num_usize || idx == order || idx == order2 {
            if idx == order {
                ei.factor[0] = f;
                f *= if enc == FILE_ENC {
                    INDICES.pawn_factor_file[(ei.norm[0] - 1) as usize][t]
                } else if enc == RANK_ENC {
                    INDICES.pawn_factor_rank[(ei.norm[0] - 1) as usize][t]
                } else if kk_enc {
                    462
                } else {
                    31332
                };
            } else if idx == order2 {
                ei.factor[ei.norm[0] as usize] = f;
                f *= subfactor(
                    ei.norm[ei.norm[0] as usize] as usize,
                    48 - (ei.norm[0] as usize),
                );
            } else {
                ei.factor[k] = f;
                f *= subfactor(ei.norm[k] as usize, n);
                n -= ei.norm[k] as usize;
                k += ei.norm[k] as usize;
            }
            idx += 1;
        }
        f
    }
}

pub unsafe fn setup_pairs(
    data: &[u8],
    offset: usize,
    tb_size: usize,
    flags: &mut u8,
    tb_type: usize,
) -> (Box<PairsData>, usize, [usize; 3]) {
    *flags = data[offset];

    if (data[offset] & 0x80) != 0 {
        let mut d = Box::new(PairsData::default());
        d.idx_bits = 0;
        d.const_value[0] = if tb_type == WDL { data[offset + 1] } else { 0 };
        d.const_value[1] = 0;
        return (d, offset + 2, [0, 0, 0]);
    }

    let block_size = data[offset + 1];
    let idx_bits = data[offset + 2];

    let real_num_blocks = u32::from_le_bytes([
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ]);
    let num_blocks = real_num_blocks + (data[offset + 3] as u32);
    let max_len = data[offset + 8] as usize;
    let min_len = data[offset + 9] as usize;
    let h = max_len - min_len + 1;

    let num_syms =
        u16::from_le_bytes([data[offset + 10 + 2 * h], data[offset + 11 + 2 * h]]) as usize;

    let mut d = Box::new(PairsData::default());
    d.block_size = block_size;
    d.idx_bits = idx_bits;
    d.min_len = min_len as u8;

    d.offset = data[offset + 10..].as_ptr() as *const u16;
    d.sym_pat = data[offset + 12 + 2 * h..].as_ptr();

    d.sym_len = vec![0; num_syms];
    d.base = vec![0; h];

    let new_offset = offset + 12 + 2 * h + 3 * num_syms + (num_syms & 1);
    let num_indices = (tb_size + (1 << idx_bits) - 1) >> idx_bits;

    let sizes = [
        6 * num_indices,
        2 * (num_blocks as usize),
        (real_num_blocks as usize) << block_size,
    ];

    unsafe {
        d.base[h - 1] = 0;
        for i in (0..h - 1).rev() {
            let off_i = u16::from_le_bytes(*(d.offset.add(i) as *const [u8; 2]));
            let off_i1 = u16::from_le_bytes(*(d.offset.add(i + 1) as *const [u8; 2]));
            d.base[i] = (d.base[i + 1] + (off_i as u64) - (off_i1 as u64)) / 2;
        }

        for i in 0..h {
            d.base[i] <<= 64 - (min_len + i);
        }

        let mut seen = vec![false; num_syms];
        for s in 0..num_syms {
            if !seen[s] {
                calc_sym_len(&mut d, s, &mut seen);
            }
        }
    }

    (d, new_offset, sizes)
}

unsafe fn calc_sym_len(d: &mut PairsData, s: usize, seen: &mut [bool]) {
    unsafe {
        if s >= seen.len() {
            return;
        }

        let w = d.sym_pat.add(3 * s);
        let s2 = ((*w.add(2) as usize) << 4) | ((*w.add(1) as usize) >> 4);

        if s2 == 0x0fff {
            d.sym_len[s] = 0;
        } else {
            let s1 = (((*w.add(1) as usize) & 0xf) << 8) | (*w as usize);
            if !seen[s1] {
                calc_sym_len(d, s1, seen);
            }
            if !seen[s2] {
                calc_sym_len(d, s2, seen);
            }
            let l1 = if s1 < d.sym_len.len() {
                d.sym_len[s1]
            } else {
                0
            };
            let l2 = if s2 < d.sym_len.len() {
                d.sym_len[s2]
            } else {
                0
            };
            d.sym_len[s] = l1.wrapping_add(l2).wrapping_add(1);
        }

        seen[s] = true;
    }
}

pub unsafe fn load_tb_tables(file_name: &str) -> Option<BaseEntry> {
    unsafe {
        let path = find_tb_file(&format!("{}.rtbw", file_name))?;
        let tb_data = TbData::new(&path).ok()?;
        let ptr = tb_data.ptr();
        let total_len = tb_data.len();

        let magic = u32::from_le_bytes([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]);
        if magic != 0x5d23e871 {
            return None;
        }

        let split = (*ptr.add(4) & 0x01) != 0;
        let mut data = ptr.add(5);

        let mut be = BaseEntry::default();
        be.key = crate::syzygy::tbchess::calc_key_from_name(file_name, false);
        be.sym_key = crate::syzygy::tbchess::calc_key_from_name(file_name, true);
        be.symmetric = be.key == be.sym_key;

        let mut w_pawns = 0;
        let mut b_pawns = 0;
        let mut past_v = false;
        for c in file_name.chars() {
            if c == 'v' {
                past_v = true;
                continue;
            }
            be.num += 1;
            if c == 'P' {
                if past_v {
                    b_pawns += 1;
                } else {
                    w_pawns += 1;
                }
            }
        }

        be.has_pawns = w_pawns > 0 || b_pawns > 0;
        if be.has_pawns {
            be.pawns[0] = w_pawns;
            be.pawns[1] = b_pawns;
            if b_pawns > 0 && (w_pawns == 0 || w_pawns > b_pawns) {
                be.pawns.swap(0, 1);
            }
        } else {
            let mut singles = 0;
            let mut counts: std::collections::HashMap<(bool, char), u8> =
                std::collections::HashMap::new();
            let mut white = true;
            for c in file_name.chars() {
                if c == 'v' {
                    white = false;
                    continue;
                }
                *counts.entry((white, c)).or_insert(0) += 1;
            }
            for v in counts.values() {
                if *v == 1 {
                    singles += 1;
                }
            }
            be.kk_enc = singles == 2;
        }

        let num_tables = if be.has_pawns { 4 } else { 1 };
        let enc = if !be.has_pawns { PIECE_ENC } else { FILE_ENC };
        let table_count = if split { num_tables * 2 } else { num_tables };

        for _ in 0..table_count {
            be.ei.push(EncInfo::default());
        }

        let mut sizes = vec![[0usize; 3]; table_count];
        let mut tb_size = vec![[0usize; 2]; num_tables];

        for t in 0..num_tables {
            tb_size[t][0] =
                init_enc_info(&mut be.ei[t], be.pawns, be.num, be.kk_enc, data, 0, t, enc);
            if split {
                tb_size[t][1] = init_enc_info(
                    &mut be.ei[num_tables + t],
                    be.pawns,
                    be.num,
                    be.kk_enc,
                    data,
                    4,
                    t,
                    enc,
                );
            }
            data = data.add(
                be.num as usize
                    + 1
                    + if be.has_pawns && be.pawns[1] > 0 {
                        1
                    } else {
                        0
                    },
            );
        }

        if (data as usize) & 1 != 0 {
            data = data.add(1);
        }

        for t in 0..num_tables {
            let mut flags = 0;
            let remaining = total_len - (data as usize - ptr as usize);
            let (d0, off0, s0) = setup_pairs(
                std::slice::from_raw_parts(data, remaining),
                0,
                tb_size[t][0],
                &mut flags,
                WDL,
            );
            be.ei[t].precomp = Some(d0);
            sizes[t] = s0;
            data = data.add(off0);

            if split {
                let remaining = total_len - (data as usize - ptr as usize);
                let (d1, off1, s1) = setup_pairs(
                    std::slice::from_raw_parts(data, remaining),
                    0,
                    tb_size[t][1],
                    &mut flags,
                    WDL,
                );
                be.ei[num_tables + t].precomp = Some(d1);
                sizes[num_tables + t] = s1;
                data = data.add(off1);
            }
        }

        for t in 0..num_tables {
            if let Some(ref mut p) = be.ei[t].precomp {
                p.index_table = data;
                data = data.add(sizes[t][0]);
            }
            if split {
                if let Some(ref mut p) = be.ei[num_tables + t].precomp {
                    p.index_table = data;
                    data = data.add(sizes[num_tables + t][0]);
                }
            }
        }
        for t in 0..num_tables {
            if let Some(ref mut p) = be.ei[t].precomp {
                p.size_table = data as *const u16;
                data = data.add(sizes[t][1]);
            }
            if split {
                if let Some(ref mut p) = be.ei[num_tables + t].precomp {
                    p.size_table = data as *const u16;
                    data = data.add(sizes[num_tables + t][1]);
                }
            }
        }
        for t in 0..num_tables {
            if let Some(ref mut p) = be.ei[t].precomp {
                let align = (data as usize + 0x3f) & !0x3f;
                data = align as *const u8;
                p.data = data;
                data = data.add(sizes[t][2]);
            }
            if split {
                if let Some(ref mut p) = be.ei[num_tables + t].precomp {
                    let align = (data as usize + 0x3f) & !0x3f;
                    data = align as *const u8;
                    p.data = data;
                    data = data.add(sizes[num_tables + t][2]);
                }
            }
        }

        be.mmap[WDL] = Some(tb_data);

        // --- Try Loading DTZ ---
        if let Some(dtz_path) = find_tb_file(&format!("{}.rtbz", file_name)) {
            if let Ok(dtz_tb) = TbData::new(&dtz_path) {
                let dtz_ptr = dtz_tb.ptr();
                let dtz_total_len = dtz_tb.len();
                let magic = u32::from_le_bytes([
                    *dtz_ptr,
                    *dtz_ptr.add(1),
                    *dtz_ptr.add(2),
                    *dtz_ptr.add(3),
                ]);

                if magic == 0xa50c66d7 {
                    let mut data = dtz_ptr.add(5);

                    for _ in 0..num_tables {
                        be.ei_dtz.push(EncInfo::default());
                    }

                    let mut tb_size = vec![0usize; num_tables];
                    for t in 0..num_tables {
                        tb_size[t] = init_enc_info(
                            &mut be.ei_dtz[t],
                            be.pawns,
                            be.num,
                            be.kk_enc,
                            data,
                            0,
                            t,
                            enc,
                        );
                        data = data.add(
                            be.num as usize
                                + 1
                                + if be.has_pawns && be.pawns[1] > 0 {
                                    1
                                } else {
                                    0
                                },
                        );
                    }

                    if (data as usize) & 1 != 0 {
                        data = data.add(1);
                    }

                    let mut sizes = vec![[0usize; 3]; num_tables];
                    for t in 0..num_tables {
                        let mut flags = 0;
                        let remaining = dtz_total_len - (data as usize - dtz_ptr as usize);
                        let (d, off, s) = setup_pairs(
                            std::slice::from_raw_parts(data, remaining),
                            0,
                            tb_size[t],
                            &mut flags,
                            DTZ,
                        );
                        be.ei_dtz[t].precomp = Some(d);
                        sizes[t] = s;
                        if !be.has_pawns {
                            be.piece_dtz_flags = flags;
                        } else {
                            be.pawn_dtz_flags[t] = flags;
                        }
                        data = data.add(off);
                    }

                    be.dtz_map = data as usize;
                    for t in 0..num_tables {
                        let flags = if !be.has_pawns {
                            be.piece_dtz_flags
                        } else {
                            be.pawn_dtz_flags[t]
                        };
                        if (flags & 2) != 0 {
                            if (flags & 16) == 0 {
                                for i in 0..4 {
                                    let map_idx = if !be.has_pawns {
                                        &mut be.piece_dtz_map_idx[i]
                                    } else {
                                        &mut be.pawn_dtz_map_idx[t][i]
                                    };
                                    *map_idx = (data as usize + 1 - be.dtz_map) as u16;
                                    data = data.add(1 + (*data as usize));
                                }
                            } else {
                                if (data as usize) & 1 != 0 {
                                    data = data.add(1);
                                }
                                for i in 0..4 {
                                    let map_idx = if !be.has_pawns {
                                        &mut be.piece_dtz_map_idx[i]
                                    } else {
                                        &mut be.pawn_dtz_map_idx[t][i]
                                    };
                                    *map_idx = ((data as usize + 2 - be.dtz_map) / 2) as u16;
                                    let len = u16::from_le_bytes([*data, *data.add(1)]) as usize;
                                    data = data.add(2 + 2 * len);
                                }
                            }
                        }
                    }

                    if (data as usize) & 1 != 0 {
                        data = data.add(1);
                    }

                    for t in 0..num_tables {
                        if let Some(ref mut p) = be.ei_dtz[t].precomp {
                            p.index_table = data;
                            data = data.add(sizes[t][0]);
                        }
                    }
                    for t in 0..num_tables {
                        if let Some(ref mut p) = be.ei_dtz[t].precomp {
                            p.size_table = data as *const u16;
                            data = data.add(sizes[t][1]);
                        }
                    }
                    for t in 0..num_tables {
                        if let Some(ref mut p) = be.ei_dtz[t].precomp {
                            let align = (data as usize + 0x3f) & !0x3f;
                            data = align as *const u8;
                            p.data = data;
                            data = data.add(sizes[t][2]);
                        }
                    }

                    be.has_dtz = true;
                    be.mmap[DTZ] = Some(dtz_tb);
                }
            }
        }

        be.ready = true;
        Some(be)
    }
}

pub fn leading_pawn(p: &mut [u8], be: &BaseEntry, enc: usize) -> usize {
    for i in 1..(be.pawns[0] as usize) {
        if FLAP[enc - 1][p[0] as usize] > FLAP[enc - 1][p[i] as usize] {
            p.swap(0, i);
        }
    }
    if enc == FILE_ENC {
        FILE_TO_FILE[(p[0] & 7) as usize] as usize
    } else {
        ((p[0] - 8) >> 3) as usize
    }
}

pub fn encode(p: &mut [u8], ei: &EncInfo, be: &BaseEntry, enc: usize) -> usize {
    let n = be.num as usize;
    let mut idx: usize;
    let mut k: usize;

    if (p[0] & 0x04) != 0 {
        for i in 0..n {
            p[i] ^= 0x07;
        }
    }

    if enc == PIECE_ENC {
        if (p[0] & 0x20) != 0 {
            for i in 0..n {
                p[i] ^= 0x38;
            }
        }

        for i in 0..n {
            if OFF_DIAG[p[i] as usize] != 0 {
                if OFF_DIAG[p[i] as usize] > 0 && i < if be.kk_enc { 2 } else { 3 } {
                    for j in 0..n {
                        p[j] = FLIP_DIAG[p[j] as usize];
                    }
                }
                break;
            }
        }

        if be.kk_enc {
            idx = KK_IDX[TRIANGLE[p[0] as usize] as usize][p[1] as usize] as usize;
            k = 2;
        } else {
            let s1 = usize::from(p[1] > p[0]);
            let s2 = usize::from(p[2] > p[0]) + usize::from(p[2] > p[1]);

            if OFF_DIAG[p[0] as usize] != 0 {
                idx = (TRIANGLE[p[0] as usize] as usize) * 63 * 62
                    + ((p[1] as usize) - s1) * 62
                    + ((p[2] as usize) - s2);
            } else if OFF_DIAG[p[1] as usize] != 0 {
                idx = 6 * 63 * 62
                    + (DIAG[p[0] as usize] as usize) * 28 * 62
                    + (LOWER[p[1] as usize] as usize) * 62
                    + (p[2] as usize)
                    - s2;
            } else if OFF_DIAG[p[2] as usize] != 0 {
                idx = 6 * 63 * 62
                    + 4 * 28 * 62
                    + (DIAG[p[0] as usize] as usize) * 7 * 28
                    + ((DIAG[p[1] as usize] as usize) - s1) * 28
                    + (LOWER[p[2] as usize] as usize);
            } else {
                idx = 6 * 63 * 62
                    + 4 * 28 * 62
                    + 4 * 7 * 28
                    + (DIAG[p[0] as usize] as usize) * 7 * 6
                    + ((DIAG[p[1] as usize] as usize) - s1) * 6
                    + ((DIAG[p[2] as usize] as usize) - s2);
            }
            k = 3;
        }
        idx *= ei.factor[0];
    } else {
        for i in 1..(be.pawns[0] as usize) {
            for j in (i + 1)..(be.pawns[0] as usize) {
                if PAWN_TWIST[enc - 1][p[i] as usize] < PAWN_TWIST[enc - 1][p[j] as usize] {
                    p.swap(i, j);
                }
            }
        }

        k = be.pawns[0] as usize;
        idx = INDICES.pawn_idx[enc - 1][k - 1][FLAP[enc - 1][p[0] as usize] as usize];
        for i in 1..k {
            idx += INDICES.binomial[k - i][PAWN_TWIST[enc - 1][p[i] as usize] as usize];
        }
        idx *= ei.factor[0];

        if be.pawns[1] > 0 {
            let t = k + be.pawns[1] as usize;
            for i in k..t {
                for j in (i + 1)..t {
                    if p[i] > p[j] {
                        p.swap(i, j);
                    }
                }
            }
            let mut s = 0usize;
            for i in k..t {
                let sq = p[i] as usize;
                let mut skips = 0;
                for j in 0..k {
                    if sq > p[j] as usize {
                        skips += 1;
                    }
                }
                let tw = sq as isize - skips as isize - 8;
                if (0..64).contains(&tw) {
                    s += INDICES.binomial[i - k + 1][tw as usize];
                }
            }
            idx += s * ei.factor[k];
            k = t;
        }
    }

    while k < n {
        let t = k + ei.norm[k] as usize;
        for i in k..t {
            for j in (i + 1)..t {
                if p[i] > p[j] {
                    p.swap(i, j);
                }
            }
        }
        let mut sub_idx = 0usize;
        for j in k..t {
            let sq = p[j] as usize;
            let mut skips = 0;
            for i in 0..k {
                if (p[i] as usize) < sq {
                    skips += 1;
                }
            }
            sub_idx += INDICES.binomial[j - k + 1][sq - skips];
        }
        idx += sub_idx * ei.factor[k];
        k = t;
    }

    idx
}

pub unsafe fn decompress_pairs(d: &PairsData, idx: usize) -> &[u8] {
    unsafe {
        if d.idx_bits == 0 {
            return &d.const_value;
        }

        let main_idx = (idx >> d.idx_bits) as u32;
        let mut lit_idx = (idx & ((1 << d.idx_bits) - 1)) as i32 - (1 << (d.idx_bits - 1));

        let block_ptr = d.index_table.add((6 * main_idx) as usize);
        let mut block = u32::from_le_bytes([
            *block_ptr,
            *block_ptr.add(1),
            *block_ptr.add(2),
            *block_ptr.add(3),
        ]);
        let idx_offset = u16::from_le_bytes([*block_ptr.add(4), *block_ptr.add(5)]);
        lit_idx += idx_offset as i32;

        let mut iterations = 0;
        if lit_idx < 0 {
            while lit_idx < 0 && block > 0 && iterations < 10000 {
                block -= 1;
                lit_idx += (*d.size_table.add(block as usize) as i32) + 1;
                iterations += 1;
            }
        } else {
            while iterations < 10000 {
                let size_val = *d.size_table.add(block as usize) as i32;
                if lit_idx <= size_val {
                    break;
                }
                lit_idx -= size_val + 1;
                block += 1;
                iterations += 1;
            }
        }

        let mut ptr = d.data.add((block as usize) << d.block_size) as *const u32;
        let m = d.min_len as usize;

        let mut code = u64::from_be_bytes(*(ptr as *const [u8; 8]));
        ptr = ptr.add(2);
        let mut bit_cnt = 0;

        let mut sym: u32;
        loop {
            let mut l = m;
            while l - m < d.base.len() && code < d.base[l - m] {
                l += 1;
            }

            let offset_val = u16::from_le_bytes(*(d.offset.add(l - m) as *const [u8; 2])) as u32;
            sym = offset_val + ((code - d.base[l - m]) >> (64 - l)) as u32;

            if lit_idx < (d.sym_len[sym as usize] as i32) + 1 {
                break;
            }

            lit_idx -= (d.sym_len[sym as usize] as i32) + 1;
            code <<= l;
            bit_cnt += l;

            if bit_cnt >= 32 {
                bit_cnt -= 32;
                let tmp = u32::from_be_bytes(*(ptr as *const [u8; 4]));
                ptr = ptr.add(1);
                code |= (tmp as u64) << bit_cnt;
            }
        }

        while d.sym_len[sym as usize] != 0 {
            let w = d.sym_pat.add(3 * (sym as usize));
            let s1 = (((*w.add(1) & 0xf) as u32) << 8) | (*w as u32);
            if lit_idx < (d.sym_len[s1 as usize] as i32) + 1 {
                sym = s1;
            } else {
                lit_idx -= (d.sym_len[s1 as usize] as i32) + 1;
                sym = ((*w.add(2) as u32) << 4) | ((*w.add(1) as u32) >> 4);
            }
        }
        std::slice::from_raw_parts(d.sym_pat.add(3 * (sym as usize)), 3)
    }
}

fn fill_squares(
    board: &Board,
    pieces: &[u8; TB_PIECES],
    flip: bool,
    mirror: u8,
    p: &mut [u8; TB_PIECES],
    mut i: usize,
) -> usize {
    if i >= TB_PIECES {
        return i;
    }

    let pc = pieces[i];
    let color = if (pc >> 3) == 0 {
        Piece::WHITE
    } else {
        Piece::BLACK
    };
    let color = if flip {
        color ^ (Piece::WHITE | Piece::BLACK)
    } else {
        color
    };

    let piece_type = Piece::get_type(pc);

    let mut bb = board.bb(color | piece_type);
    let start = i;
    while bb != 0 && i < TB_PIECES {
        p[i] = bb.trailing_zeros() as u8 ^ mirror;
        i += 1;
        bb &= bb - 1;
    }

    if i == start { i + 1 } else { i }
}

fn get_tb_entry(board: &Board, key: u64) -> Option<Arc<BaseEntry>> {
    if let Some(cached) = TB_CACHE.read().unwrap().get(&key) {
        return cached.clone();
    }

    for file in [get_tb_filename(board, false), get_tb_filename(board, true)] {
        if let Some(be) = unsafe { load_tb_tables(&file) } {
            let entry = Arc::new(be);
            let mut cache = TB_CACHE.write().unwrap();
            cache.insert(entry.key, Some(entry.clone()));
            cache.insert(entry.sym_key, Some(entry.clone()));
            return Some(entry);
        }
    }

    let mut cache = TB_CACHE.write().unwrap();
    cache.insert(key, None);
    None
}

pub unsafe fn probe_wdl_table(board: &Board, success: &mut i32) -> i32 {
    let key = crate::syzygy::tbchess::calc_key(board, false);

    if key == 0 {
        *success = 1;
        return 0;
    }

    let be = match get_tb_entry(board, key) {
        Some(be) => be,
        None => {
            *success = 0;
            return 0;
        }
    };

    if !be.ready {
        *success = 0;
        return 0;
    }

    let (flip, bside) = if !be.symmetric {
        let flip = key != be.key;
        (flip, (board.side_to_move() == Color::White) == flip)
    } else {
        (board.side_to_move() != Color::White, false)
    };

    let mut p = [0u8; TB_PIECES];

    let ei_idx = if !be.has_pawns {
        let idx = if bside { 1 } else { 0 };
        let mut i = 0;
        while i < be.num as usize {
            i = fill_squares(board, &be.ei[idx].pieces, flip, 0, &mut p, i);
        }
        idx
    } else {
        let mut i = fill_squares(board, &be.ei[0].pieces, flip, flip_mask(flip), &mut p, 0);
        let t = leading_pawn(&mut p, &be, FILE_ENC);
        let idx = t + if bside { 4 } else { 0 };
        while i < be.num as usize {
            i = fill_squares(board, &be.ei[idx].pieces, flip, flip_mask(flip), &mut p, i);
        }
        idx
    };

    let ei = &be.ei[ei_idx];

    let enc_type = if !be.has_pawns { PIECE_ENC } else { FILE_ENC };
    let idx = encode(&mut p, ei, &be, enc_type);

    unsafe {
        let Some(precomp) = ei.precomp.as_ref() else {
            *success = 0;
            return 0;
        };
        let w = decompress_pairs(precomp, idx);

        *success = 1;
        w[0] as i32 - 2
    }
}

pub unsafe fn probe_dtz_table(board: &Board, wdl: i32, success: &mut i32) -> i32 {
    let key = crate::syzygy::tbchess::calc_key(board, false);

    if key == 0 {
        *success = 1;
        return 0;
    }

    let be = match get_tb_entry(board, key) {
        Some(be) => be,
        None => {
            *success = 0;
            return 0;
        }
    };

    if !be.ready || !be.has_dtz {
        *success = -1;
        return 0;
    }

    let (flip, bside) = if !be.symmetric {
        let flip = key != be.key;
        (flip, (board.side_to_move() == Color::White) == flip)
    } else {
        (board.side_to_move() != Color::White, false)
    };

    let mut p = [0u8; TB_PIECES];
    let ei_idx;
    let mut t = 0;

    if !be.has_pawns {
        let flags = be.piece_dtz_flags;
        if (flags & 1) != (if bside { 1 } else { 0 }) && !be.symmetric {
            *success = -1;
            return 0;
        }
        ei_idx = 0;
        let mut i = 0;
        while i < be.num as usize {
            i = fill_squares(board, &be.ei_dtz[ei_idx].pieces, flip, 0, &mut p, i);
        }
    } else {
        let mut i = fill_squares(
            board,
            &be.ei_dtz[0].pieces,
            flip,
            flip_mask(flip),
            &mut p,
            0,
        );
        t = leading_pawn(&mut p, &be, FILE_ENC);
        let flags = be.pawn_dtz_flags[t];
        if (flags & 1) != (if bside { 1 } else { 0 }) && !be.symmetric {
            *success = -1;
            return 0;
        }
        ei_idx = t;
        while i < be.num as usize {
            i = fill_squares(
                board,
                &be.ei_dtz[ei_idx].pieces,
                flip,
                flip_mask(flip),
                &mut p,
                i,
            );
        }
    }

    let ei = &be.ei_dtz[ei_idx];
    let enc_type = if !be.has_pawns { PIECE_ENC } else { FILE_ENC };
    let idx = encode(&mut p, ei, &be, enc_type);

    unsafe {
        let precomp = ei.precomp.as_ref().unwrap();
        let w = decompress_pairs(precomp, idx);

        let mut v = w[0] as i32 + (((w[1] & 0x0f) as i32) << 8);
        let flags = if !be.has_pawns {
            be.piece_dtz_flags
        } else {
            be.pawn_dtz_flags[t]
        };

        if (flags & 2) != 0 {
            let m = WDL_TO_MAP[(wdl + 2) as usize];
            let map_idx = if !be.has_pawns {
                be.piece_dtz_map_idx[m]
            } else {
                be.pawn_dtz_map_idx[t][m]
            };

            if (flags & 16) == 0 {
                v = *((be.dtz_map as *const u8).add(map_idx as usize + v as usize)) as i32;
            } else {
                let ptr = (be.dtz_map as *const u8)
                    .cast::<u16>()
                    .add(map_idx as usize + v as usize);
                let bytes = std::slice::from_raw_parts(ptr as *const u8, 2);
                v = u16::from_le_bytes([bytes[0], bytes[1]]) as i32;
            }
        }

        if (flags & PA_FLAGS[(wdl + 2) as usize]) == 0 || (wdl & 1) != 0 {
            v *= 2;
        }

        *success = 1;
        v
    }
}

#[inline(always)]
fn flip_mask(flip: bool) -> u8 {
    if flip { 0x38 } else { 0 }
}

pub fn probe_ab(board: &mut Board, mut alpha: i32, beta: i32, success: &mut i32) -> i32 {
    let mut moves = board.gen_cap_moves();
    board.filter_illegal(&mut moves);

    for i in 0..moves.len() {
        let mv = moves.moves[i];
        let undo = board.make_move(&mv);

        let mut ab_success = 1;
        let v = -probe_ab(board, -beta, -alpha, &mut ab_success);
        board.unmake_move(&mv, &undo);

        if ab_success == 0 {
            *success = 0;
            return 0;
        }
        if v > alpha {
            if v >= beta {
                return v;
            }
            alpha = v;
        }
    }

    let mut tb_success = 1;
    let v = unsafe { probe_wdl_table(board, &mut tb_success) };
    if tb_success == 0 {
        *success = 0;
        return 0;
    }

    if alpha >= v { alpha } else { v }
}

pub fn probe_wdl(board: &mut Board, success: &mut i32) -> i32 {
    *success = 1;

    let mut moves = board.gen_cap_moves();
    board.filter_illegal(&mut moves);

    let mut best_cap = -3;
    let mut best_ep = -3;

    for i in 0..moves.len() {
        let mv = moves.moves[i];
        let undo = board.make_move(&mv);

        let mut ab_success = 1;
        let v = -probe_ab(board, -2, -best_cap, &mut ab_success);

        board.unmake_move(&mv, &undo);

        if ab_success == 0 {
            *success = 0;
            return 0;
        }

        if v > best_cap {
            if v == 2 {
                *success = 2;
                return 2;
            }
            if mv.flag() != MoveFlag::EN_PASSANT {
                best_cap = v;
            } else if v > best_ep {
                best_ep = v;
            }
        }
    }

    let v = unsafe { probe_wdl_table(board, success) };
    if *success == 0 {
        return 0;
    }

    if best_ep > best_cap {
        if best_ep > v {
            *success = 2;
            return best_ep;
        }
        best_cap = best_ep;
    }

    if best_cap >= v {
        *success = 1 + if best_cap > 0 { 1 } else { 0 };
        return best_cap;
    }

    if best_ep > -3 && v == 0 {
        let mut all_moves = board.gen_moves();
        board.filter_illegal(&mut all_moves);
        let has_other_move =
            (0..all_moves.len()).any(|i| all_moves.moves[i].flag() != MoveFlag::EN_PASSANT);

        if !has_other_move && !board.in_check() {
            *success = 2;
            return best_ep;
        }
    }

    v
}

pub fn probe_dtz(board: &mut Board, success: &mut i32) -> i32 {
    let wdl = probe_wdl(board, success);
    if *success == 0 {
        return 0;
    }

    if wdl == 0 {
        return 0;
    }

    if *success == 2 {
        return WDL_TO_DTZ[(wdl + 2) as usize];
    }

    if wdl > 0 {
        let mut moves = board.gen_moves();
        board.filter_illegal(&mut moves);

        for i in 0..moves.len() {
            let mv = moves.moves[i];
            let piece = board.piece_on(mv.from());
            let is_capture = mv.flag().is_capture() || mv.flag() == MoveFlag::EN_PASSANT;

            if Piece::get_type(piece) != Piece::PAWN || is_capture {
                continue;
            }

            let undo = board.make_move(&mv);
            let mut p_success = 1;
            let v = -probe_wdl(board, &mut p_success);
            board.unmake_move(&mv, &undo);

            if p_success == 0 {
                *success = 0;
                return 0;
            }
            if v == wdl {
                return WDL_TO_DTZ[(wdl + 2) as usize];
            }
        }
    }

    let dtz = unsafe { probe_dtz_table(board, wdl, success) };
    if *success > 0 {
        return WDL_TO_DTZ[(wdl + 2) as usize] + if wdl > 0 { dtz } else { -dtz };
    }

    let mut best = if wdl > 0 {
        i32::MAX
    } else {
        WDL_TO_DTZ[(wdl + 2) as usize]
    };

    let mut moves = board.gen_moves();
    board.filter_illegal(&mut moves);

    for i in 0..moves.len() {
        let mv = moves.moves[i];
        let piece = board.piece_on(mv.from());
        let is_capture = mv.flag().is_capture() || mv.flag() == MoveFlag::EN_PASSANT;

        if is_capture || Piece::get_type(piece) == Piece::PAWN {
            continue;
        }

        let undo = board.make_move(&mv);
        let mut p_success = 1;
        let v = -probe_dtz(board, &mut p_success);

        if v == 1 && is_mate(board) {
            best = 1;
        } else if wdl > 0 {
            if v > 0 && v + 1 < best {
                best = v + 1;
            }
        } else {
            if v - 1 < best {
                best = v - 1;
            }
        }

        board.unmake_move(&mv, &undo);

        if p_success == 0 {
            *success = 0;
            return 0;
        }
    }

    *success = 1;
    best
}

fn is_mate(board: &mut Board) -> bool {
    if !board.in_check() {
        return false;
    }
    let mut moves = board.gen_moves();
    board.filter_illegal(&mut moves);
    moves.len() == 0
}

pub fn root_probe_wdl(board: &mut Board, use_rule50: bool, rm: &mut TbRootMoves) -> bool {
    const WDL_TO_RANK: [i32; 5] = [-1000, -899, 0, 899, 1000];
    let wdl_to_value: [i32; 5] = [
        -MATE as i32 + MAX_PLY as i32 + 1,
        -2,
        0,
        2,
        MATE as i32 - MAX_PLY as i32 - 1,
    ];

    let mut moves = board.gen_moves();
    board.filter_illegal(&mut moves);
    rm.moves.clear();

    if moves.len() == 0 {
        return false;
    }

    for i in 0..moves.len() {
        let mv = moves.moves[i];
        let undo = board.make_move(&mv);

        let mut success = 1;
        let mut v = -probe_wdl(board, &mut success);
        board.unmake_move(&mv, &undo);

        if success == 0 {
            return false;
        }

        if !use_rule50 {
            v = if v > 0 {
                2
            } else if v < 0 {
                -2
            } else {
                0
            };
        }

        rm.moves.push(TbRootMove {
            mv,
            pv: vec![mv],
            tb_rank: WDL_TO_RANK[(v + 2) as usize],
            tb_score: wdl_to_value[(v + 2) as usize],
        });
    }
    true
}

pub fn root_probe_dtz(
    board: &mut Board,
    has_repeated: bool,
    use_rule50: bool,
    rm: &mut TbRootMoves,
) -> bool {
    let cnt50 = board.halfmove_clock() as i32;
    let bound = if use_rule50 { 900 } else { 1 };

    let mut moves = board.gen_moves();
    board.filter_illegal(&mut moves);
    rm.moves.clear();

    if moves.len() == 0 {
        return false;
    }

    for i in 0..moves.len() {
        let mv = moves.moves[i];

        // Determine whether this move resets the fifty-move counter
        // BEFORE making the move.
        let piece = board.piece_on(mv.from());
        let is_zeroing = Piece::get_type(piece) == Piece::PAWN
            || mv.flag().is_capture()
            || mv.flag() == MoveFlag::EN_PASSANT;

        let undo = board.make_move(&mv);
        let mut success = 1;
        let mut v;

        if is_zeroing {
            // Zeroing moves are evaluated using WDL.
            v = -probe_wdl(board, &mut success);

            if success == 0 {
                board.unmake_move(&mv, &undo);
                return false;
            }

            v = WDL_TO_DTZ[(v + 2) as usize];
        } else {
            // Non-zeroing moves are evaluated using DTZ.
            v = -probe_dtz(board, &mut success);

            if success == 0 {
                board.unmake_move(&mv, &undo);
                return false;
            }

            // Account for the root move itself.
            if v > 0 {
                v += 1;
            } else if v < 0 {
                v -= 1;
            }
        }

        // Handle direct checkmates.
        if is_mate(board) {
            if v > 0 {
                v = 1;
            } else if v < 0 {
                v = -1;
            }
        }

        board.unmake_move(&mv, &undo);

        // Calculate the rank, accounting for the fifty-move rule
        // and possible repetition.
        let r = if v > 0 {
            if v == 1 {
                2000
            } else if v + cnt50 <= 99 && !has_repeated {
                1000 - v
            } else {
                1000 - (v + cnt50)
            }
        } else if v < 0 {
            if v == -1 {
                -2000
            } else if -v * 2 + cnt50 < 100 {
                -1000 - v
            } else {
                -1000 + (-v + cnt50)
            }
        } else {
            0
        };

        let score = if r >= bound {
            MATE as i32 - MAX_PLY as i32 + 1
        } else if r > -bound {
            0
        } else {
            -MATE as i32 + MAX_PLY as i32 - 1
        };

        rm.moves.push(TbRootMove {
            mv,
            pv: vec![mv],
            tb_rank: r,
            tb_score: score,
        });
    }

    true
}
