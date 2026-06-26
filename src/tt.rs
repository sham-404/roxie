use crate::items::Move;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u16,
    pub score: i32,
    pub flag: TTFlag,
    pub best_move: Move,
}

impl TTEntry {
    fn default() -> Self {
        Self {
            key: 0,
            depth: 0,
            score: 0,
            flag: TTFlag::LowerBound,
            best_move: Move::NULL,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TTBucket {
    pub slots: [TTEntry; 2],
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

        Self {
            table: vec![
                TTBucket {
                    slots: [TTEntry::default(); 2]
                };
                num_buckets
            ],
            mask: num_buckets - 1,
        }
    }

    pub fn probe(&self, key: u64) -> Option<&TTEntry> {
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
        let score_0 = replace_score(bucket.slots[0].depth, bucket.slots[0].flag);
        let score_new = replace_score(new_entry.depth, new_entry.flag);

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
        TTFlag::Exact => 3,       // PV nodes
        TTFlag::LowerBound => 2,  // Beta cutoffs
        TTFlag::UpperBound => 0,  // Fail-low nodes
    };
    
    // Combine depth and flag quality
    (depth as i32) + flag_bonus
}
