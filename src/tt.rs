use crate::{items::Move, search::MATE, uci_print};

const TT_SLOT_SIZE: usize = 3;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TTFlag {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

impl TTFlag {
    #[inline]
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

    #[inline]
    pub fn flag(&self) -> TTFlag {
        let bits = ((self.info & TTPacked::FLAG_MASK) >> TTPacked::FLAG_SHIFT) as u8;
        TTFlag::from_bits(bits)
    }

    #[inline]
    pub fn depth(&self) -> u16 {
        ((self.info & TTPacked::DEPTH_MASK) >> TTPacked::DEPTH_SHIFT) as u16
    }

    #[inline]
    pub fn best_move(&self) -> Move {
        let mv = ((self.info & TTPacked::MOVE_MASK) >> TTPacked::MOVE_SHIFT) as u16;
        Move(mv)
    }

    #[inline]
    pub fn score(&self) -> i16 {
        let u16_score = ((self.info & TTPacked::SCORE_MASK) >> TTPacked::SCORE_SHIFT) as u16;
        let de_offset_score = u16_score as i32 - MATE as i32;
        de_offset_score as i16
    }

    #[inline]
    pub fn age(&self) -> u8 {
        ((self.info & TTPacked::AGE_MASK) >> TTPacked::AGE_SHIFT) as u8
    }

    fn default() -> Self {
        Self { key: 0, info: 0 }
    }
}

#[derive(Clone, Copy)]
pub struct TTBucket {
    pub slots: [TTPacked; TT_SLOT_SIZE],
}

pub struct TranspositionTable {
    table: Vec<TTBucket>,
    generation: u8,
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

        Self {
            table: vec![
                TTBucket {
                    slots: [TTPacked::default(); TT_SLOT_SIZE]
                };
                num_buckets
            ],
            generation: 0,
            mask: num_buckets - 1,
        }
    }

    #[inline]
    pub fn info(&self) {
        let bucket_size = std::mem::size_of::<TTBucket>();
        uci_print!(
            "info string Hash Table initialized with {} entries taking {} MB space",
            self.table.len() * TT_SLOT_SIZE,
            self.table.len() * bucket_size / (1024 * 1024)
        );
    }

    pub fn probe(&self, key: u64) -> Option<&TTPacked> {
        let index = key as usize & self.mask;
        let bucket = &self.table[index];

        for slot in &bucket.slots {
            if slot.key == key {
                return Some(slot);
            }
        }

        None
    }

    pub fn store(&mut self, new_entry: TTEntry) {
        let new_packed = TTPacked::new(new_entry);
        let index = new_packed.key as usize & self.mask;
        let bucket = &mut self.table[index];

        let mut victim_idx = 0;
        let mut lowest_score = i32::MAX;

        for i in 0..TT_SLOT_SIZE {
            let slot = &bucket.slots[i];

            // filling empty slots immediately
            if slot.key == 0 {
                bucket.slots[i] = new_packed;
                return;
            }

            // exact key match
            if slot.key == new_packed.key {
                if new_packed.depth() >= slot.depth() {
                    bucket.slots[i] = new_packed;
                }
                return;
            }

            // track the victim
            let score = replace_score(slot, self.generation);
            if score < lowest_score {
                lowest_score = score;
                victim_idx = i;
            }
        }

        // Overwrite the worst node in the bucket
        bucket.slots[victim_idx] = new_packed;
    }

    #[inline]
    pub fn inc_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    #[inline]
    pub fn get_generation(&self) -> u8 {
        self.generation
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
