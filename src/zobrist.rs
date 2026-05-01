#[derive(Clone)]
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { s: seed }
    }

    pub fn rand64(&mut self) -> u64 {
        self.s ^= self.s >> 12;
        self.s ^= self.s << 25;
        self.s ^= self.s >> 27;
        self.s.wrapping_mul(0x2545F4914F6CDD1)
    }

    pub fn sparse_rand64(&mut self) -> u64 {
        self.rand64() & self.rand64() & self.rand64()
    }
}

use std::sync::OnceLock;

pub const NPIECES: usize = 12;

pub static ZOBRIST_TABLE: OnceLock<[[u64; 64]; NPIECES]> = OnceLock::new();
pub static ENPASSANT_KEYS: OnceLock<[u64; 8]> = OnceLock::new();
pub static CASTLING_KEYS: OnceLock<[u64; 16]> = OnceLock::new();
pub static SIDE_KEY: OnceLock<u64> = OnceLock::new();

pub fn init_zobrist() {
    let mut rng = Rng::new(14974698296094900119);

    ZOBRIST_TABLE.get_or_init(|| {
        let mut table = [[0u64; 64]; NPIECES];
        for p in 0..NPIECES {
            for sq in 0..64 {
                table[p][sq] = rng.rand64();
            }
        }
        table
    });

    ENPASSANT_KEYS.get_or_init(|| {
        let mut ep = [0u64; 8];
        for i in 0..8 {
            ep[i] = rng.rand64();
        }
        ep
    });

    CASTLING_KEYS.get_or_init(|| {
        let mut castling = [0u64; 16];
        for i in 0..16 {
            castling[i] = rng.rand64();
        }
        castling
    });

    SIDE_KEY.get_or_init(|| rng.rand64());
}
