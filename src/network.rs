use std::{
    fs::{File, metadata},
    io::{Cursor, Read, Seek},
    sync::OnceLock,
};

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "avx2"
))]
use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

use crate::{
    board::{Board, pop_lsb},
    r#const::{BLACK, WHITE},
    engine::Engine,
    items::{Color, Move, MoveFlag, Piece, Undo},
};

pub const INPUT: usize = 6144;
pub const HL1: usize = 256;
pub const HL2: usize = 16;
pub const HL3: usize = 32;
pub const OUTPUT: usize = 1;
pub const MAGIC: &[u8; 8] = b"BLAZE_V#";
pub const NN_DATA: &[u8] = include_bytes!("blaze.nnue");

pub const QP: i32 = 8;
pub const Q: f32 = (1 << QP) as f32; // 256.0

#[rustfmt::skip]
pub const KING_BUCKETS: [usize; 64] = [
    0, 0, 1, 1, 1, 1, 0, 0,
    0, 0, 1, 1, 1, 1, 0, 0,
    2, 2, 3, 3, 3, 3, 2, 2,
    2, 2, 3, 3, 3, 3, 2, 2,
    4, 4, 5, 5, 5, 5, 4, 4,
    4, 4, 5, 5, 5, 5, 4, 4,
    6, 6, 7, 7, 7, 7, 6, 6,
    6, 6, 7, 7, 7, 7, 6, 6,
];

#[rustfmt::skip]
pub const MIRROR_MASK: [u8; 64] = [
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
    0, 0, 0, 0, 7, 7, 7, 7,
];

pub static NETWORK: OnceLock<Network> = OnceLock::new();

pub fn init_nn(is_needed: bool) {
    if !is_needed {
        return;
    }

    NETWORK.get_or_init(|| {
        let mut reader = Cursor::new(NN_DATA);
        let nn = Network::load(&mut reader);
        assert_eq!(reader.position() as usize, NN_DATA.len());
        nn
    });
}

pub struct Network {
    w1: Vec<i16>,
    b1: Vec<i16>,

    w2: Vec<i16>,
    b2: Vec<i16>,

    w3: Vec<i16>,
    b3: Vec<i16>,

    w4: Vec<i16>,
    b4: Vec<i16>,
}

impl Network {
    pub fn load(reader: &mut impl Read) -> Network {
        let mut magic = [0u8; MAGIC.len()];
        reader.read_exact(&mut magic).unwrap();
        assert_eq!(
            &magic, MAGIC,
            "Magic header mismatch! Ensure this is the f32 export."
        );

        let mut read_i16 = |size: usize| -> Vec<i16> {
            let mut byte_buffer = vec![0u8; size * 2];
            reader.read_exact(&mut byte_buffer).unwrap();

            let mut i16_data = Vec::with_capacity(size);
            for chunk in byte_buffer.chunks_exact(2) {
                i16_data.push(i16::from_le_bytes([chunk[0], chunk[1]]));
            }
            i16_data
        };

        Network {
            w1: read_i16(INPUT * HL1),
            b1: read_i16(HL1),
            w2: read_i16(HL1 * 2 * HL2),
            b2: read_i16(HL2),
            w3: read_i16(HL2 * HL3),
            b3: read_i16(HL3),
            w4: read_i16(HL3 * OUTPUT),
            b4: read_i16(OUTPUT),
        }
    }

    fn read_f32(reader: &mut impl Read, size: usize) -> Vec<f32> {
        let mut bytes = vec![0u8; size * 4];
        reader.read_exact(&mut bytes).unwrap();
        let mut out = Vec::with_capacity(size);
        for chunk in bytes.chunks_exact(4) {
            out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        out
    }

    pub fn load_unquantized(path: &str) -> Network {
        let mut file = File::open(path).unwrap();
        let file_size = metadata(path).unwrap().len();

        let mut magic = [0u8; MAGIC.len()];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(
            &magic, MAGIC,
            "Magic header mismatch! Ensure this is the f32 export."
        );

        let w1 = Network::quantize_to_i16(&Network::read_f32(&mut file, INPUT * HL1));
        let b1 = Network::quantize_to_i16(&Network::read_f32(&mut file, HL1));

        let w2_raw = Network::quantize_to_i16(&Network::read_f32(&mut file, HL1 * 2 * HL2));
        let w2 = Network::transpose_weights(&w2_raw, HL1 * 2, HL2);
        let b2 = Network::quantize_to_i16(&Network::read_f32(&mut file, HL2));

        let w3_raw = Network::quantize_to_i16(&Network::read_f32(&mut file, HL2 * HL3));
        let w3 = Network::transpose_weights(&w3_raw, HL2, HL3);
        let b3 = Network::quantize_to_i16(&Network::read_f32(&mut file, HL3));

        let w4_raw = Network::quantize_to_i16(&Network::read_f32(&mut file, HL3 * OUTPUT));
        let w4 = Network::transpose_weights(&w4_raw, HL3, OUTPUT);
        let b4 = Network::quantize_to_i16(&Network::read_f32(&mut file, OUTPUT));

        let pos = file.stream_position().unwrap();
        assert_eq!(
            file_size, pos,
            "Did not reach end of file! File size mismatch."
        );

        Network {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
            w4,
            b4,
        }
    }

    fn transpose_weights(weights: &[i16], input_len: usize, output_len: usize) -> Vec<i16> {
        let mut transposed = vec![0; input_len * output_len];
        for out_idx in 0..output_len {
            for in_idx in 0..input_len {
                transposed[out_idx * input_len + in_idx] = weights[in_idx * output_len + out_idx];
            }
        }
        transposed
    }

    fn quantize_to_i16(layer: &[f32]) -> Vec<i16> {
        let mut quantized: Vec<i16> = Vec::with_capacity(layer.len());
        for &val in layer {
            quantized.push((val * Q).round() as i16);
        }
        quantized
    }

    #[inline(always)]
    pub fn feature_w(piece_idx: usize, sq: usize, wk_sq: usize) -> usize {
        let wb = KING_BUCKETS[wk_sq];
        let w_mask = MIRROR_MASK[wk_sq] as usize;
        wb * 768 + piece_idx * 64 + (sq ^ w_mask)
    }

    #[inline(always)]
    pub fn feature_b(piece_idx: usize, sq: usize, bk_sq: usize) -> usize {
        let bk_sq_flip = bk_sq ^ 56;
        let bb_bucket = KING_BUCKETS[bk_sq_flip];
        let b_mask = MIRROR_MASK[bk_sq_flip] as usize;
        let b_idx = (piece_idx + 6) % 12;
        let b_sq = (sq ^ 56) ^ b_mask;
        bb_bucket * 768 + b_idx * 64 + b_sq
    }

    pub fn build_acc_white(&self, board: &Board, wk_sq: usize) -> [i16; HL1] {
        let mut acc = [0; HL1];
        acc.copy_from_slice(&self.b1);
        let bb = board.get_bb();
        for (idx, &bitboard) in bb.iter().enumerate() {
            let mut piece_bb = bitboard;
            while let Some(sq) = pop_lsb(&mut piece_bb) {
                let feat = Network::feature_w(idx, sq, wk_sq);
                let offset = feat * HL1;
                for i in 0..HL1 {
                    acc[i] += self.w1[offset + i];
                }
            }
        }
        acc
    }

    pub fn build_acc_black(&self, board: &Board, bk_sq: usize) -> [i16; HL1] {
        let mut acc = [0; HL1];
        acc.copy_from_slice(&self.b1);
        let bb = board.get_bb();
        for (idx, &bitboard) in bb.iter().enumerate() {
            let mut piece_bb = bitboard;
            while let Some(sq) = pop_lsb(&mut piece_bb) {
                let feat = Network::feature_b(idx, sq, bk_sq);
                let offset = feat * HL1;
                for i in 0..HL1 {
                    acc[i] += self.w1[offset + i];
                }
            }
        }
        acc
    }

    pub fn build_acc(&self, board: &Board) -> [[i16; HL1]; 2] {
        let mut wk_bb = board.bb(Piece::WHITE | Piece::KING);
        let mut bk_bb = board.bb(Piece::BLACK | Piece::KING);
        let wk_sq = pop_lsb(&mut wk_bb).expect("No king present!");
        let bk_sq = pop_lsb(&mut bk_bb).expect("No king present!");

        let mut acc = [[0; HL1]; 2];
        acc[WHITE] = self.build_acc_white(board, wk_sq);
        acc[BLACK] = self.build_acc_black(board, bk_sq);
        acc
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    pub fn apply_single_update(acc: &mut [i16; HL1], w: &[i16], feat: usize, remove: bool) {
        unsafe { Self::apply_single_update_avx2(acc, w, feat, remove) };
    }

    #[cfg(target_arch = "aarch64")]
    pub fn apply_single_update(acc: &mut [i16; HL1], w: &[i16], feat: usize, remove: bool) {
        unsafe { Self::apply_single_update_neon(acc, w, feat, remove) };
    }

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        target_arch = "aarch64"
    )))]
    pub fn apply_single_update(acc: &mut [i16; HL1], w: &[i16], feat: usize, remove: bool) {
        Self::apply_single_update_scalar(acc, w, feat, remove);
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    #[target_feature(enable = "avx2")]
    unsafe fn apply_single_update_avx2(acc: &mut [i16; HL1], w: &[i16], feat: usize, remove: bool) {
        unsafe {
            let mut i = 0;
            let acc_ptr = acc.as_mut_ptr();
            let w_ptr = w.as_ptr().add(feat * HL1);

            while i < HL1 {
                let a = _mm256_loadu_si256(acc_ptr.add(i) as *const __m256i);
                let b = _mm256_loadu_si256(w_ptr.add(i) as *const __m256i);
                let res = if remove {
                    _mm256_sub_epi16(a, b)
                } else {
                    _mm256_add_epi16(a, b)
                };
                _mm256_storeu_si256(acc_ptr.add(i) as *mut __m256i, res);
                i += 16;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe fn apply_single_update_neon(acc: &mut [i16; HL1], w: &[i16], feat: usize, remove: bool) {
        unsafe {
            let mut i = 0;
            let acc_ptr = acc.as_mut_ptr();
            let w_ptr = w.as_ptr().add(feat * HL1);

            while i < HL1 {
                let a = vld1q_s16(acc_ptr.add(i));
                let b = vld1q_s16(w_ptr.add(i));
                let res = if remove {
                    vsubq_s16(a, b)
                } else {
                    vaddq_s16(a, b)
                };
                vst1q_s16(acc_ptr.add(i), res);
                i += 8;
            }
        }
    }

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        target_arch = "aarch64"
    )))]
    fn apply_single_update_scalar(acc: &mut [i16; HL1], w: &[i16], feat: usize, remove: bool) {
        let offset = feat * HL1;
        for i in 0..HL1 {
            if remove {
                acc[i] -= w[offset + i];
            } else {
                acc[i] += w[offset + i];
            }
        }
    }

    pub fn eval_with_acc(&self, buf: &mut EvalBuf, acc: &[i16]) -> f32 {
        buf.fc1.copy_from_slice(acc);

        Network::screlu(&mut buf.fc1);

        Network::process_layer(&buf.fc1, &mut buf.fc2, &self.w2, &self.b2, true);
        Network::screlu(&mut buf.fc2);

        Network::process_layer(&buf.fc2, &mut buf.fc3, &self.w3, &self.b3, true);
        Network::screlu(&mut buf.fc3);

        Network::process_layer(&buf.fc3, &mut buf.fc4, &self.w4, &self.b4, true);

        (buf.fc4[0] as f32 / Q) * 400.0
    }

    pub fn evaluate(&self, board: &Board) -> f32 {
        let bb = board.get_bb();

        let mut wk_bb = board.bb(Piece::WHITE | Piece::KING);
        let mut bk_bb = board.bb(Piece::BLACK | Piece::KING);
        let wk_sq = pop_lsb(&mut wk_bb).unwrap_or(0);
        let bk_sq = pop_lsb(&mut bk_bb).unwrap_or(0);

        let wb = KING_BUCKETS[wk_sq];
        let w_mask = MIRROR_MASK[wk_sq] as usize;

        let bk_sq_flip = bk_sq ^ 56;
        let bb_bucket = KING_BUCKETS[bk_sq_flip];
        let b_mask = MIRROR_MASK[bk_sq_flip] as usize;

        let mut w_acc = self.b1.clone();
        let mut b_acc = self.b1.clone();

        // building accumulators
        for (idx, &bitboard) in bb.iter().enumerate() {
            let mut piece_bb = bitboard;
            while let Some(sq) = pop_lsb(&mut piece_bb) {
                // --- White Perspective ---
                let w_sq = sq ^ w_mask;
                let w_idx = wb * 768 + idx * 64 + w_sq;
                let w_weights = &self.w1[w_idx * 768..(w_idx + 1) * 768];
                for i in 0..768 {
                    w_acc[i] += w_weights[i];
                }

                let b_idx = (idx + 6) % 12;
                let b_sq = (sq ^ 56) ^ b_mask;
                let b_feat_idx = bb_bucket * 768 + b_idx * 64 + b_sq;
                let b_weights = &self.w1[b_feat_idx * 768..(b_feat_idx + 1) * 768];
                for i in 0..768 {
                    b_acc[i] += b_weights[i];
                }
            }
        }

        let mut buf = EvalBuf::new();
        let is_white = board.side_to_move() == Color::White;
        if is_white {
            buf.fc1[..HL1].copy_from_slice(&w_acc);
            buf.fc1[HL1..].copy_from_slice(&b_acc);
        } else {
            buf.fc1[..HL1].copy_from_slice(&b_acc);
            buf.fc1[HL1..].copy_from_slice(&w_acc);
        };

        Network::screlu(&mut buf.fc1);

        Network::process_layer(&buf.fc1, &mut buf.fc2, &self.w2, &self.b2, true);
        Network::screlu(&mut buf.fc2);

        Network::process_layer(&buf.fc2, &mut buf.fc3, &self.w3, &self.b3, true);
        Network::screlu(&mut buf.fc3);

        Network::process_layer(&buf.fc3, &mut buf.fc4, &self.w4, &self.b4, true);

        (buf.fc4[0] as f32 / Q) * 400.0
    }

    // ==============================================================================
    // SIMD SCReLU
    // ==============================================================================
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    pub fn screlu(layer: &mut [i16]) {
        unsafe { Self::screlu_avx2(layer) };
    }

    #[cfg(target_arch = "aarch64")]
    pub fn screlu(layer: &mut [i16]) {
        unsafe { Self::screlu_neon(layer) };
    }

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        target_arch = "aarch64"
    )))]
    pub fn screlu(layer: &mut [i16]) {
        Self::screlu_scalar(layer);
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    #[target_feature(enable = "avx2")]
    unsafe fn screlu_avx2(layer: &mut [i16]) {
        unsafe {
            let len = layer.len();
            let mut i = 0;
            let v_min = _mm256_setzero_si256();
            let v_max = _mm256_set1_epi16(Q as i16);

            while i + 16 <= len {
                let ptr = layer.as_mut_ptr().add(i);
                let mut v = _mm256_loadu_si256(ptr as *const __m256i);

                v = _mm256_max_epi16(v, v_min);
                v = _mm256_min_epi16(v, v_max);

                let v_lo = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(v));
                let v_hi = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(v, 1));

                let sq_lo = _mm256_mullo_epi32(v_lo, v_lo);
                let sq_hi = _mm256_mullo_epi32(v_hi, v_hi);

                let sh_lo = _mm256_srai_epi32(sq_lo, 8);
                let sh_hi = _mm256_srai_epi32(sq_hi, 8);

                let packed = _mm256_packs_epi32(sh_lo, sh_hi);
                let res = _mm256_permute4x64_epi64(packed, 0xD8);

                _mm256_storeu_si256(ptr as *mut __m256i, res);
                i += 16;
            }

            while i < len {
                let u = (*layer.get_unchecked(i) as i32).clamp(0, Q as i32);
                *layer.get_unchecked_mut(i) = ((u * u) >> QP) as i16;
                i += 1;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe fn screlu_neon(layer: &mut [i16]) {
        unsafe {
            let len = layer.len();
            let mut i = 0;
            let v_min = vdupq_n_s16(0);
            let v_max = vdupq_n_s16(Q as i16);

            while i + 8 <= len {
                let ptr = layer.as_mut_ptr().add(i);
                let mut v = vld1q_s16(ptr);

                v = vmaxq_s16(v, v_min);
                v = vminq_s16(v, v_max);

                let v_lo = vmovl_s16(vget_low_s16(v));
                let v_hi = vmovl_s16(vget_high_s16(v));

                let sq_lo = vmulq_s32(v_lo, v_lo);
                let sq_hi = vmulq_s32(v_hi, v_hi);

                let sh_lo = vshrq_n_s32::<8>(sq_lo);
                let sh_hi = vshrq_n_s32::<8>(sq_hi);

                let res = vcombine_s16(vqmovn_s32(sh_lo), vqmovn_s32(sh_hi));
                vst1q_s16(ptr, res);

                i += 8;
            }
            while i < len {
                let u = (*layer.get_unchecked(i) as i32).clamp(0, Q as i32);
                *layer.get_unchecked_mut(i) = ((u * u) >> QP) as i16;
                i += 1;
            }
        }
    }

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        target_arch = "aarch64"
    )))]
    fn screlu_scalar(layer: &mut [i16]) {
        let q_int = Q as i32;
        let len = layer.len();
        let mut idx = 0;
        while idx < len {
            unsafe {
                let u = (*layer.get_unchecked(idx) as i32).clamp(0, q_int);
                *layer.get_unchecked_mut(idx) = ((u * u) >> QP) as i16;
            }
            idx += 1;
        }
    }

    // ==============================================================================
    // SIMD LINEAR LAYER
    // ==============================================================================
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    pub fn process_layer(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        unsafe { Self::process_layer_avx2(inp_layer, out_layer, weight, bias, to_quantize) }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn process_layer(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        unsafe { Self::process_layer_neon(inp_layer, out_layer, weight, bias, to_quantize) }
    }

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        target_arch = "aarch64"
    )))]
    pub fn process_layer(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        Self::process_layer_scalar(inp_layer, out_layer, weight, bias, to_quantize)
    }

    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "avx2"
    ))]
    #[target_feature(enable = "avx2")]
    unsafe fn process_layer_avx2(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        let input_len = inp_layer.len();
        let out_len = bias.len();

        unsafe {
            for neuron_idx in 0..out_len {
                let w_offset = neuron_idx * input_len;
                let mut i = 0;
                let mut acc = _mm256_setzero_si256();

                while i + 16 <= input_len {
                    let inp = _mm256_loadu_si256(inp_layer.as_ptr().add(i) as *const __m256i);
                    let w = _mm256_loadu_si256(weight.as_ptr().add(w_offset + i) as *const __m256i);

                    let prod = _mm256_madd_epi16(inp, w);
                    acc = _mm256_add_epi32(acc, prod);
                    i += 16;
                }

                let acc_128 = _mm_add_epi32(
                    _mm256_castsi256_si128(acc),
                    _mm256_extracti128_si256(acc, 1),
                );
                let mut sums = [0i32; 4];
                _mm_storeu_si128(sums.as_mut_ptr() as *mut __m128i, acc_128);
                let mut dot = sums[0] + sums[1] + sums[2] + sums[3];

                while i < input_len {
                    dot += (*inp_layer.get_unchecked(i) as i32)
                        * (*weight.get_unchecked(w_offset + i) as i32);
                    i += 1;
                }

                let b = *bias.get_unchecked(neuron_idx) as i32;
                let val = b + if to_quantize {
                    (dot + (1 << (QP - 1))) >> QP
                } else {
                    dot
                };

                *out_layer.get_unchecked_mut(neuron_idx) = val.clamp(-32768, 32767) as i16;
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe fn process_layer_neon(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        let input_len = inp_layer.len();
        let out_len = bias.len();

        unsafe {
            for neuron_idx in 0..out_len {
                let w_offset = neuron_idx * input_len;
                let mut i = 0;
                let mut acc = vdupq_n_s32(0);

                while i + 8 <= input_len {
                    let inp = vld1q_s16(inp_layer.as_ptr().add(i));
                    let w = vld1q_s16(weight.as_ptr().add(w_offset + i));

                    acc = vmlal_s16(acc, vget_low_s16(inp), vget_low_s16(w));
                    acc = vmlal_high_s16(acc, inp, w);

                    i += 8;
                }

                let mut dot = vaddvq_s32(acc);

                while i < input_len {
                    dot += (*inp_layer.get_unchecked(i) as i32)
                        * (*weight.get_unchecked(w_offset + i) as i32);
                    i += 1;
                }

                let b = *bias.get_unchecked(neuron_idx) as i32;
                let val = b + if to_quantize {
                    (dot + (1 << (QP - 1))) >> QP
                } else {
                    dot
                };
                *out_layer.get_unchecked_mut(neuron_idx) = val.clamp(-32768, 32767) as i16;
            }
        }
    }

    #[cfg(not(any(
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "avx2"
        ),
        target_arch = "aarch64"
    )))]
    fn process_layer_scalar(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        let input_len = inp_layer.len();
        let out_len = bias.len();

        for neuron_idx in 0..out_len {
            let mut dot: i32 = 0;
            let w_offset = neuron_idx * input_len;

            unsafe {
                for i in 0..input_len {
                    let inp = *inp_layer.get_unchecked(i) as i32;
                    let w = *weight.get_unchecked(w_offset + i) as i32;
                    dot += inp * w;
                }

                let b = *bias.get_unchecked(neuron_idx) as i32;
                let val = b + if to_quantize {
                    (dot + (1 << (QP - 1))) >> QP
                } else {
                    dot
                };

                *out_layer.get_unchecked_mut(neuron_idx) = val.clamp(-32768, 32767) as i16;
            }
        }
    }
}

impl Engine {
    pub fn update_nnue(&mut self, mv: &Move, undo: &Undo, ply: usize) {
        let Some(nn) = NETWORK.get() else {
            return;
        };

        *self.accumulators.get_mut(ply + 1) = self.accumulators.get(ply);
        let acc = self.accumulators.get_mut(ply + 1);

        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        // Note: update_nnue is called AFTER make_move, so the piece is already at `to`
        let moved_piece = self.board.piece_on(to);
        let pt = Piece::get_type(moved_piece);
        let side = Piece::get_color(moved_piece);

        // Fetch King squares safely directly from the bitboards
        let w_king = pop_lsb(&mut self.board.bb(Piece::WHITE | Piece::KING))
            .expect("Where is the king dude??");
        let b_king = pop_lsb(&mut self.board.bb(Piece::BLACK | Piece::KING))
            .expect("Where is the king dude??");

        let mut do_full_w = false;
        let mut do_full_b = false;

        // Check if king moved across the bucket
        if pt == Piece::KING {
            let is_castle = (from as i32 - to as i32).abs() == 2;
            if is_castle {
                do_full_w = true;
                do_full_b = true;
            } else {
                if side == Piece::WHITE {
                    if KING_BUCKETS[from] != KING_BUCKETS[to]
                        || MIRROR_MASK[from] != MIRROR_MASK[to]
                    {
                        do_full_w = true;
                    }
                } else {
                    let o = from ^ 56;
                    let n = to ^ 56;
                    if KING_BUCKETS[o] != KING_BUCKETS[n] || MIRROR_MASK[o] != MIRROR_MASK[n] {
                        do_full_b = true;
                    }
                }
            }
        }

        // registering added and removed features for incremental update
        let mut w_removed = [0usize; 3];
        let mut b_removed = [0usize; 3];
        let mut w_added = [0usize; 3];
        let mut b_added = [0usize; 3];
        let mut r_cnt = 0;
        let mut a_cnt = 0;

        let piece_idx = Piece::to_idx(moved_piece);

        if flag.is_promo() {
            let pawn_idx = Piece::to_idx(Piece::PAWN | side);
            w_removed[r_cnt] = Network::feature_w(pawn_idx, from, w_king);
            b_removed[r_cnt] = Network::feature_b(pawn_idx, from, b_king);
            r_cnt += 1;
        } else {
            w_removed[r_cnt] = Network::feature_w(piece_idx, from, w_king);
            b_removed[r_cnt] = Network::feature_b(piece_idx, from, b_king);
            r_cnt += 1;
        }

        w_added[a_cnt] = Network::feature_w(piece_idx, to, w_king);
        b_added[a_cnt] = Network::feature_b(piece_idx, to, b_king);
        a_cnt += 1;

        if flag.is_capture() {
            let cap_sq = if flag == MoveFlag::EN_PASSANT {
                if side == Piece::WHITE { to - 8 } else { to + 8 }
            } else {
                to
            };

            let cap_piece = undo.captured;
            let cap_idx = Piece::to_idx(cap_piece);

            w_removed[r_cnt] = Network::feature_w(cap_idx, cap_sq, w_king);
            b_removed[r_cnt] = Network::feature_b(cap_idx, cap_sq, b_king);
            r_cnt += 1;
        }

        if do_full_w {
            acc[WHITE].copy_from_slice(&nn.build_acc_white(&self.board, w_king));
        } else {
            for i in 0..a_cnt {
                Network::apply_single_update(&mut acc[WHITE], &nn.w1, w_added[i], false);
            }
            for i in 0..r_cnt {
                Network::apply_single_update(&mut acc[WHITE], &nn.w1, w_removed[i], true);
            }
        }

        if do_full_b {
            acc[BLACK].copy_from_slice(&nn.build_acc_black(&self.board, b_king));
        } else {
            for i in 0..a_cnt {
                Network::apply_single_update(&mut acc[BLACK], &nn.w1, b_added[i], false);
            }
            for i in 0..r_cnt {
                Network::apply_single_update(&mut acc[BLACK], &nn.w1, b_removed[i], true);
            }
        }
    }

    pub fn update_nnue_null_move(&mut self, ply: usize) {
        *self.accumulators.get_mut(ply + 1) = self.accumulators.get(ply)
    }
}

pub struct EvalBuf {
    fc1: [i16; HL1 * 2],
    fc2: [i16; HL2],
    fc3: [i16; HL3],
    fc4: [i16; OUTPUT],
}

impl EvalBuf {
    pub fn new() -> EvalBuf {
        EvalBuf {
            fc1: [0; HL1 * 2],
            fc2: [0; HL2],
            fc3: [0; HL3],
            fc4: [0; OUTPUT],
        }
    }
}
