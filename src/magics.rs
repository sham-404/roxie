use std::sync::OnceLock;

use crate::{
    board::{BISHOP_DIRS, ROOK_DIRS, mask},
    square::Square,
};

// Mask needed for magic indexing for rook
#[rustfmt::skip]
const ROOK_MASKS: [u64; 64] = [
    0x101010101017e,    0x202020202027c,    0x404040404047a,    0x8080808080876,
    0x1010101010106e,   0x2020202020205e,   0x4040404040403e,   0x8080808080807e,
    0x1010101017e00,    0x2020202027c00,    0x4040404047a00,    0x8080808087600,
    0x10101010106e00,   0x20202020205e00,   0x40404040403e00,   0x80808080807e00,
    0x10101017e0100,    0x20202027c0200,    0x40404047a0400,    0x8080808760800,
    0x101010106e1000,   0x202020205e2000,   0x404040403e4000,   0x808080807e8000,
    0x101017e010100,    0x202027c020200,    0x404047a040400,    0x8080876080800,
    0x1010106e101000,   0x2020205e202000,   0x4040403e404000,   0x8080807e808000,
    0x1017e01010100,    0x2027c02020200,    0x4047a04040400,    0x8087608080800,
    0x10106e10101000,   0x20205e20202000,   0x40403e40404000,   0x80807e80808000,
    0x17e0101010100,    0x27c0202020200,    0x47a0404040400,    0x8760808080800,
    0x106e1010101000,   0x205e2020202000,   0x403e4040404000,   0x807e8080808000,
    0x7e010101010100,   0x7c020202020200,   0x7a040404040400,   0x76080808080800,
    0x6e101010101000,   0x5e202020202000,   0x3e404040404000,   0x7e808080808000,
    0x7e01010101010100, 0x7c02020202020200, 0x7a04040404040400, 0x7608080808080800,
    0x6e10101010101000, 0x5e20202020202000, 0x3e40404040404000, 0x7e80808080808000,
];

// Mask needed for magic indexing for bishop
#[rustfmt::skip]
const BISHOP_MASKS: [u64; 64] = [
    0x40201008040200,   0x402010080400,     0x4020100a00,       0x40221400,
    0x2442800,          0x204085000,        0x20408102000,      0x2040810204000,
    0x20100804020000,   0x40201008040000,   0x4020100a0000,     0x4022140000,
    0x244280000,        0x20408500000,      0x2040810200000,    0x4081020400000,
    0x10080402000200,   0x20100804000400,   0x4020100a000a00,   0x402214001400,
    0x24428002800,      0x2040850005000,    0x4081020002000,    0x8102040004000,
    0x8040200020400,    0x10080400040800,   0x20100a000a1000,   0x40221400142200,
    0x2442800284400,    0x4085000500800,    0x8102000201000,    0x10204000402000,
    0x4020002040800,    0x8040004081000,    0x100a000a102000,   0x22140014224000,
    0x44280028440200,   0x8500050080400,    0x10200020100800,   0x20400040201000,
    0x2000204081000,    0x4000408102000,    0xa000a10204000,    0x14001422400000,
    0x28002844020000,   0x50005008040200,   0x20002010080400,   0x40004020100800,
    0x20408102000,      0x40810204000,      0xa1020400000,      0x142240000000,
    0x284402000000,     0x500804020000,     0x201008040200,     0x402010080400,
    0x2040810204000,    0x4081020400000,    0xa102040000000,    0x14224000000000,
    0x28440200000000,   0x50080402000000,   0x20100804020000,   0x40201008040200,
];

// Magics are took from the Lambergar engine
#[rustfmt::skip]
pub const ROOK_MAGICS: [u64; 64] = [
    0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
    0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
    0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
    0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
    0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
    0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
    0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
    0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
    0x0000204000800080, 0x0000200040401000, 0x0000100080802000, 0x0000080080801000,
    0x0000040080800800, 0x0000020080800400, 0x0000020001010004, 0x0000800040800100,
    0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000800100020080, 0x0000800041000080,
    0x00FFFCDDFCED714A, 0x007FFCDDFCED714A, 0x003FFFCDFFD88096, 0x0000040810002101,
    0x0001000204080011, 0x0001000204000801, 0x0001000082000401, 0x0001FFFAABFAD1A2,
];

#[rustfmt::skip]
pub const BISHOP_MAGICS: [u64; 64] = [
    0x0002020202020200, 0x0002020202020000, 0x0004010202000000, 0x0004040080000000,
    0x0001104000000000, 0x0000821040000000, 0x0000410410400000, 0x0000104104104000,
    0x0000040404040400, 0x0000020202020200, 0x0000040102020000, 0x0000040400800000,
    0x0000011040000000, 0x0000008210400000, 0x0000004104104000, 0x0000002082082000,
    0x0004000808080800, 0x0002000404040400, 0x0001000202020200, 0x0000800802004000,
    0x0000800400A00000, 0x0000200100884000, 0x0000400082082000, 0x0000200041041000,
    0x0002080010101000, 0x0001040008080800, 0x0000208004010400, 0x0000404004010200,
    0x0000840000802000, 0x0000404002011000, 0x0000808001041000, 0x0000404000820800,
    0x0001041000202000, 0x0000820800101000, 0x0000104400080800, 0x0000020080080080,
    0x0000404040040100, 0x0000808100020100, 0x0001010100020800, 0x0000808080010400,
    0x0000820820004000, 0x0000410410002000, 0x0000082088001000, 0x0000002011000800,
    0x0000080100400400, 0x0001010101000200, 0x0002020202000400, 0x0001010101000200,
    0x0000410410400000, 0x0000208208200000, 0x0000002084100000, 0x0000000020880000,
    0x0000001002020000, 0x0000040408020000, 0x0004040404040000, 0x0002020202020000,
    0x0000104104104000, 0x0000002082082000, 0x0000000020841000, 0x0000000000208800,
    0x0000000010020200, 0x0000000404080200, 0x0000040404040400, 0x0002020202020200,
];

pub static ROOK_ATTACKS: OnceLock<Vec<Vec<u64>>> = OnceLock::new();
pub static BISHOP_ATTACKS: OnceLock<Vec<Vec<u64>>> = OnceLock::new();

pub fn init_magics() {
    ROOK_ATTACKS.get_or_init(init_rook_atk);
    BISHOP_ATTACKS.get_or_init(init_bishop_atk);
}

pub fn get_bishop_move_bits(sq: usize, occ: u64) -> u64 {
    let occ = occ & BISHOP_MASKS[sq];
    let shift = 64 - BISHOP_MASKS[sq].count_ones();

    let idx = occ.wrapping_mul(BISHOP_MAGICS[sq]) >> shift;

    BISHOP_ATTACKS.get().unwrap()[sq][idx as usize]
}

pub fn get_rook_move_bits(sq: usize, occ: u64) -> u64 {
    let occ = occ & ROOK_MASKS[sq];
    let shift = 64 - ROOK_MASKS[sq].count_ones();

    let idx = occ.wrapping_mul(ROOK_MAGICS[sq]) >> shift;

    ROOK_ATTACKS.get().unwrap()[sq][idx as usize]
}

fn get_blocker_occ(index: usize, bits: u32, mut mask: u64) -> u64 {
    let mut occ = 0;

    for i in 0..bits {
        // getting the sq idx from the mask
        let sq = mask.trailing_zeros();

        // flipping the lsb to 0
        mask &= mask - 1;

        // checking whether the idx is mapped with the cur mask bit and
        // updating the occ accodingly
        if index & (1 << i) != 0 {
            occ |= 1u64 << sq;
        }
    }
    occ
}

fn init_rook_atk() -> Vec<Vec<u64>> {
    let mut table: Vec<Vec<u64>> = Vec::new();

    for sq in 0..64 {
        let relevant_bits = ROOK_MASKS[sq].count_ones();
        let occ_count = 1 << relevant_bits; // total no of possible configurations of blockers
        table.push(vec![0; occ_count]);

        for index in 0..occ_count {
            let blocker = get_blocker_occ(index, relevant_bits, ROOK_MASKS[sq]);

            let move_bits = gen_rook_moves(sq, blocker);

            let magic_idx = blocker.wrapping_mul(ROOK_MAGICS[sq]) >> (64 - relevant_bits);
            table[sq][magic_idx as usize] = move_bits;
        }
    }
    table
}

fn init_bishop_atk() -> Vec<Vec<u64>> {
    let mut table: Vec<Vec<u64>> = Vec::new();

    for sq in 0..64 {
        let relevant_bits = BISHOP_MASKS[sq].count_ones();
        let occ_count = 1 << relevant_bits;
        table.push(vec![0; occ_count]);

        for index in 0..occ_count {
            let blocker = get_blocker_occ(index, relevant_bits, BISHOP_MASKS[sq]);

            let move_bits = gen_bishop_moves(sq, blocker);

            let magic_idx = blocker.wrapping_mul(BISHOP_MAGICS[sq]) >> (64 - relevant_bits);
            table[sq][magic_idx as usize] = move_bits;
        }
    }
    table
}

fn gen_rook_moves(sq_idx: usize, blockers: u64) -> u64 {
    let mut move_bits = 0;
    let from = Square::new(sq_idx);

    for &(dr, df) in &ROOK_DIRS {
        let mut sq = from;
        while let Some(next) = sq.offset(dr, df) {
            let to_bb = mask(next.index());

            if to_bb & blockers != 0 {
                move_bits |= to_bb;
                break;
            }

            move_bits |= to_bb;

            sq = next;
        }
    }

    move_bits
}

fn gen_bishop_moves(sq_idx: usize, blockers: u64) -> u64 {
    let mut move_bits = 0;
    let from = Square::new(sq_idx);

    for &(dr, df) in &BISHOP_DIRS {
        let mut sq = from;
        while let Some(next) = sq.offset(dr, df) {
            let to_bb = mask(next.index());

            if to_bb & blockers != 0 {
                move_bits |= to_bb;
                break;
            }

            move_bits |= to_bb;

            sq = next;
        }
    }

    move_bits
}
