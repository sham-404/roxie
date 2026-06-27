use crate::items::Move;

#[repr(u8)]
#[derive(Clone, Copy)]
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
    pub score: i32,
    info: u32,
    // NOTE: info is masked as follows:
    // 000000   0000 0000 0000 0000    0000 0000    00
    // unused   |--- best_move ---|    | depth |  |flag|
}

impl TTPacked {
    const FLAG_BITS: u32 = 2;
    const DEPTH_BITS: u32 = 8;
    const MOVE_BITS: u32 = 16;

    const FLAG_SHIFT: u32 = 0;
    const DEPTH_SHIFT: u32 = 2;
    const MOVE_SHIFT: u32 = 10;

    const FLAG_MASK: u32 = ((1 << TTPacked::FLAG_BITS) - 1) << TTPacked::FLAG_SHIFT;
    const DEPTH_MASK: u32 = ((1 << TTPacked::DEPTH_BITS) - 1) << TTPacked::DEPTH_SHIFT;
    const MOVE_MASK: u32 = ((1 << TTPacked::MOVE_BITS) - 1) << TTPacked::MOVE_SHIFT;

    pub fn new(entry: TTEntry) -> TTPacked {
        let info =
            ((entry.best_move.0 as u32) << 10) | ((entry.depth as u32) << 2) | (entry.flag as u32);

        TTPacked {
            key: entry.key,
            score: entry.score,
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

    fn default() -> Self {
        Self {
            score: 0,
            key: 0,
            info: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TTBucket {
    pub slots: [TTPacked; 2],
}

pub struct TranspositionTable {
    table: Vec<TTBucket>,
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

        println!("bucket size: {bucket_size}");
        println!("num buckets: {num_buckets}");
        println!("size: {} mb", num_buckets * bucket_size / (1024 * 1024));

        Self {
            table: vec![
                TTBucket {
                    slots: [TTPacked::default(); 2]
                };
                num_buckets
            ],
            mask: num_buckets - 1,
        }
    }

    pub fn probe(&self, key: u64) -> Option<&TTPacked> {
        let index = key as usize & self.mask;
        let bucket = &self.table[index];

        if bucket.slots[0].key == key {
            Some(&bucket.slots[0])
        } else if bucket.slots[1].key == key {
            Some(&bucket.slots[1])
        } else {
            None
        }
    }

    pub fn store(&mut self, new_entry: TTEntry) {
        let new_entry = TTPacked::new(new_entry);

        let index = new_entry.key as usize & self.mask;
        let bucket = &mut self.table[index];

        // overwriting directly if the key is same
        if bucket.slots[0].key == new_entry.key {
            bucket.slots[0] = new_entry;
            return;
        }
        if bucket.slots[1].key == new_entry.key {
            bucket.slots[1] = new_entry;
            return;
        }

        // finding relative score based on flag for replacements
        let score_0 = replace_score(bucket.slots[0].depth(), bucket.slots[0].flag());
        let score_new = replace_score(new_entry.depth(), new_entry.flag());

        if bucket.slots[0].key == 0 || score_new > score_0 {
            bucket.slots[1] = bucket.slots[0];
            bucket.slots[0] = new_entry;
        } else {
            bucket.slots[1] = new_entry;
        }
    }
}

fn replace_score(depth: u16, flag: TTFlag) -> i32 {
    let flag_bonus = match flag {
        TTFlag::Exact => 3,      // PV nodes
        TTFlag::LowerBound => 2, // Beta cutoffs
        TTFlag::UpperBound => 0, // Fail-low nodes
    };

    // Combine depth and flag quality
    ((depth as i32) << 2) + flag_bonus
}
