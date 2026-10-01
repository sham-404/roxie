use std::sync::{
    LazyLock,
    atomic::{AtomicU64, Ordering},
};

pub static TB_HASH: LazyLock<TBHash> = LazyLock::new(|| TBHash::new(4));

pub struct TBHash {
    table: Box<[AtomicU64]>,
    mask: usize,
}

impl TBHash {
    pub fn new(mb: usize) -> Self {
        let bytes = mb * 1024 * 1024;
        let entry_size = std::mem::size_of::<AtomicU64>(); // 8 bytes
        let mut num_entries = bytes / entry_size;

        num_entries = if num_entries.is_power_of_two() {
            num_entries
        } else {
            num_entries.next_power_of_two() / 2
        };

        let mut table = Vec::with_capacity(num_entries);
        for _ in 0..num_entries {
            table.push(AtomicU64::new(0));
        }

        Self {
            table: table.into_boxed_slice(),
            mask: num_entries - 1,
        }
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<(i8, bool)> {
        let index = (key as usize) & self.mask;
        let packed = self.table[index].load(Ordering::Relaxed);

        if packed == 0 {
            return None;
        }

        // Bits  0..=31: Tag (32 bits of the upper Zobrist key)
        // Bits 32..=34: WDL score offset by +2 (values 0 to 4)
        // Bit      35: Success flag (1 bit)
        let stored_tag = (packed & 0xFFFF_FFFF) as u32;
        let expected_tag = (key >> 32) as u32; // Use upper 32 bits as the tag

        if stored_tag == expected_tag {
            let wdl_encoded = ((packed >> 32) & 0x7) as i8;
            let success = ((packed >> 35) & 1) != 0;

            return Some((wdl_encoded - 2, success));
        }

        None
    }

    #[inline(always)]
    pub fn store(&self, key: u64, wdl: i8, success: bool) {
        let index = (key as usize) & self.mask;

        let tag = (key >> 32) as u32;
        let wdl_encoded = (wdl + 2) as u64 & 0x7; // Offset -2..2 to 0..4
        let success_bit = (success as u64) << 3;

        // |.............| |1 bit (35th) | | 3 bits (32 to 34) |  | 32 bits (0 to 31) |
        //  not allocated    success bit    offsetted WDL score        zobrist key
        let packed = (tag as u64) | (wdl_encoded << 32) | (success_bit << 32);

        self.table[index].store(packed, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        for entry in &self.table {
            entry.store(0, Ordering::Relaxed);
        }
    }
}
