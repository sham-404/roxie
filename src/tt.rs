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

pub struct TranspositionTable {
    table: Vec<TTEntry>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(mb: usize) -> Self {
        let bytes = mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<TTEntry>();

        let mut num_entries = bytes / entry_size;

        // round down to power of 2
        num_entries = num_entries.next_power_of_two() / 2;

        let table = vec![TTEntry::default(); num_entries];

        Self {
            table,
            mask: num_entries - 1,
        }
    }

    pub fn probe(&self, key: u64) -> Option<&TTEntry> {
        let index = key as usize & self.mask;
        let entry = &self.table[index];

        if entry.key == key { Some(entry) } else { None }
    }

    pub fn store(&mut self, new_entry: TTEntry) {
        let index = new_entry.key as usize & self.mask;
        let old_entry = &mut self.table[index];

        if old_entry.key == 0 || new_entry.depth >= old_entry.depth {
            *old_entry = new_entry;
        }
    }
}
