use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use crate::{items::Move, search::MATE, uci_print};

const TT_SLOT_SIZE: usize = 4;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TTFlag {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

impl TTFlag {
    #[inline(always)]
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Exact,
            1 => Self::LowerBound,
            2 => Self::UpperBound,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u16,
    pub score: i32,
    pub flag: TTFlag,
    pub best_move: Move,
    pub age: u8,
}

// impl TTEntry {
//     fn default() -> Self {
//         Self {
//             score: 0,
//             key: 0,
//             depth: 0,
//             flag: TTFlag::LowerBound,
//             best_move: Move::NULL,
//         }
//     }
// }

#[derive(Clone, Copy)]
pub struct TTPacked {
    key: u64,
    info: u64,
    // NOTE: info is masked as follows:
    // 0000 0....  0000 0000  0000 0000 0000 0000   0000 0000 0000 0000    0000 0000    00
    // |-unused-|  |--age--|  |----- score -----|   |--- best_move ---|    | depth |  |flag|
}

impl TTPacked {
    const FLAG_BITS: u64 = 2;
    const DEPTH_BITS: u64 = 8;
    const MOVE_BITS: u64 = 16;
    const SCORE_BITS: u64 = 16;
    const AGE_BITS: u64 = 8;

    const FLAG_SHIFT: u64 = 0;
    const DEPTH_SHIFT: u64 = 2;
    const MOVE_SHIFT: u64 = 10;
    const SCORE_SHIFT: u64 = 26;
    const AGE_SHIFT: u64 = 42;

    const FLAG_MASK: u64 = ((1 << TTPacked::FLAG_BITS) - 1) << TTPacked::FLAG_SHIFT;
    const DEPTH_MASK: u64 = ((1 << TTPacked::DEPTH_BITS) - 1) << TTPacked::DEPTH_SHIFT;
    const MOVE_MASK: u64 = ((1 << TTPacked::MOVE_BITS) - 1) << TTPacked::MOVE_SHIFT;
    const SCORE_MASK: u64 = ((1 << TTPacked::SCORE_BITS) - 1) << TTPacked::SCORE_SHIFT;
    const AGE_MASK: u64 = ((1 << TTPacked::AGE_BITS) - 1) << TTPacked::AGE_SHIFT;

    pub fn new(entry: TTEntry) -> TTPacked {
        // offsetting score as encoding direct negative values will be sign extended
        // leading to corrupt data

        let u16_score = (entry.score + MATE as i32) as u16;
        let info = ((entry.best_move.0 as u64) << TTPacked::MOVE_SHIFT)
            | ((entry.depth as u64) << TTPacked::DEPTH_SHIFT)
            | ((entry.flag as u64) << TTPacked::FLAG_SHIFT)
            | ((u16_score as u64) << TTPacked::SCORE_SHIFT)
            | ((entry.age as u64) << TTPacked::AGE_SHIFT);

        TTPacked {
            key: entry.key,
            info: info,
        }
    }

    #[inline(always)]
    pub fn flag(&self) -> TTFlag {
        let bits = ((self.info & TTPacked::FLAG_MASK) >> TTPacked::FLAG_SHIFT) as u8;
        TTFlag::from_bits(bits)
    }

    #[inline(always)]
    pub fn depth(&self) -> u16 {
        ((self.info & TTPacked::DEPTH_MASK) >> TTPacked::DEPTH_SHIFT) as u16
    }

    #[inline(always)]
    pub fn best_move(&self) -> Move {
        let mv = ((self.info & TTPacked::MOVE_MASK) >> TTPacked::MOVE_SHIFT) as u16;
        Move(mv)
    }

    #[inline(always)]
    pub fn score(&self) -> i16 {
        let u16_score = ((self.info & TTPacked::SCORE_MASK) >> TTPacked::SCORE_SHIFT) as u16;
        let de_offset_score = u16_score as i32 - MATE as i32;
        de_offset_score as i16
    }

    #[inline(always)]
    pub fn age(&self) -> u8 {
        ((self.info & TTPacked::AGE_MASK) >> TTPacked::AGE_SHIFT) as u8
    }

    // fn default() -> Self {
    //     Self { key: 0, info: 0 }
    // }
}

pub struct AtomicTTEntry {
    info: AtomicU64,
    key: AtomicU64,
}

impl AtomicTTEntry {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            info: AtomicU64::new(0),
            key: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub fn load(&self) -> TTPacked {
        let info = self.info.load(Ordering::Relaxed);
        let key = self.key.load(Ordering::Relaxed);

        TTPacked {
            key: info ^ key, // XORing key with info (to avoid retrieving fault data)
            info: info,
        }
    }

    #[inline(always)]
    pub fn store(&self, packed_entry: &TTPacked) {
        let key = packed_entry.key ^ packed_entry.info;

        self.info.store(packed_entry.info, Ordering::Relaxed);
        self.key.store(key, Ordering::Relaxed);
    }
}

#[repr(align(64))]
pub struct TTBucket {
    pub slots: [AtomicTTEntry; TT_SLOT_SIZE],
}

impl Default for TTBucket {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| AtomicTTEntry::new()),
        }
    }
}

pub struct TranspositionTable {
    table: Vec<TTBucket>,
    generation: AtomicU8,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let bytes = mb * 1024 * 1024;
        let bucket_size = std::mem::size_of::<TTBucket>();
        let mut num_buckets = bytes / bucket_size;

        num_buckets = if num_buckets.is_power_of_two() {
            num_buckets
        } else {
            num_buckets.next_power_of_two() / 2
        };

        // println!("no of buckets: {num_buckets}");
        // println!("bucket size: {bucket_size}");
        // println!("entry size: {}", bucket_size / TT_SLOT_SIZE);
        // println!("tt size: {}mb", num_buckets * bucket_size / (1024 * 1024));

        let mut table = Vec::with_capacity(num_buckets);
        for _ in 0..num_buckets {
            table.push(TTBucket::default());
        }

        Self {
            table,
            generation: AtomicU8::new(0),
            mask: num_buckets - 1,
        }
    }

    #[inline(always)]
    pub fn info(&self) {
        let bucket_size = std::mem::size_of::<TTBucket>();
        uci_print!(
            "info string Hash Table initialized with {} entries taking {} MB space",
            self.table.len() * TT_SLOT_SIZE,
            self.table.len() * bucket_size / (1024 * 1024)
        );
    }

    pub fn probe(&self, key: u64) -> Option<TTPacked> {
        let index = key as usize & self.mask;
        let bucket = &self.table[index];

        for slot in &bucket.slots {
            let packed_entry = slot.load();
            if packed_entry.key == key {
                return Some(packed_entry);
            }
        }

        None
    }

    pub fn store(&self, mut new_entry: TTEntry) {
        let mut new_packed = TTPacked::new(new_entry);
        let index = new_packed.key as usize & self.mask;
        let bucket = &self.table[index];

        let mut victim_idx = 0;
        let mut lowest_score = i32::MAX;
        let cur_gen = self.generation.load(Ordering::Relaxed);

        for i in 0..TT_SLOT_SIZE {
            let slot = &bucket.slots[i];
            let tt_entry = slot.load();

            // filling empty slots immediately
            if tt_entry.key == 0 {
                slot.store(&new_packed);
                return;
            }

            // exact key match
            if tt_entry.key == new_packed.key {
                // If Q-search is trying to store a NULL move, and we already
                // have a perfectly good move from a previous search, rescue it.
                if new_entry.best_move == Move::NULL {
                    let old_move = tt_entry.best_move();
                    if old_move != Move::NULL {
                        new_entry.best_move = old_move;
                        new_packed = TTPacked::new(new_entry); // Repack with the rescued move
                    }
                }

                // Overwrite if the new depth is greater or equal
                if new_packed.depth() >= tt_entry.depth() {
                    slot.store(&new_packed);
                }
                return;
            }

            // track the victim
            let score = replace_score(&tt_entry, cur_gen);
            if score < lowest_score {
                lowest_score = score;
                victim_idx = i;
            }
        }

        // Overwrite the worst node in the bucket
        bucket.slots[victim_idx].store(&new_packed);
    }

    #[inline(always)]
    pub fn clear(&self) {
        for bucket in self.table.iter() {
            for slot in &bucket.slots {
                slot.info.store(0, Ordering::Relaxed);
                slot.key.store(0, Ordering::Relaxed);
            }
        }
        self.generation.store(0, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn inc_generation(&self) {
        self.generation.fetch_add(1, Ordering::Release);
    }

    #[inline(always)]
    pub fn get_generation(&self) -> u8 {
        self.generation.load(Ordering::Relaxed)
    }
}

const AGE_PENALTY: i32 = 12;
fn replace_score(entry: &TTPacked, cur_gen: u8) -> i32 {
    let age = cur_gen.wrapping_sub(entry.age());

    let flag_bonus = match entry.flag() {
        TTFlag::Exact => 16,     // PV nodes
        TTFlag::LowerBound => 4, // Beta cutoffs
        TTFlag::UpperBound => 0, // Fail-low nodes
    };

    let age_pen = age as i32 * AGE_PENALTY;

    // Combine depth, flag quality, and age penalty
    ((entry.depth() as i32) << 4) + flag_bonus - age_pen
}
