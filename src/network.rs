use std::{
    fs::{File, metadata},
    io::{Cursor, Read, Seek},
    sync::OnceLock,
};

use std::arch::x86_64::*;

use crate::{
    board::{Board, pop_lsb},
    r#const::{BLACK, WHITE},
    engine::Engine,
    evaluation::mirror,
    items::{Color, Move, MoveFlag, Piece, Undo},
};

const INPUT: usize = 40960;
pub const HL1: usize = 128;
const HL2: usize = 16;
const HL3: usize = 16;
const OUTPUT: usize = 1;
const MAGIC: &[u8; 8] = b"BLAZE_V@";
const NN_DATA: &[u8] = include_bytes!("blaze.nnue");
const QP: i32 = 8;
pub const Q: f32 = (1 << QP) as f32; // 2 ^ QP

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

pub static NETWORK: OnceLock<Network> = OnceLock::new();

pub fn init_nn(is_needed: bool) {
    if !is_needed {
        return;
    }
    NETWORK.get_or_init(|| {
        let mut reader = Cursor::new(NN_DATA);
        let nn = Network::load(&mut reader);
        assert_eq!(reader.position() as usize, NN_DATA.len()); // ensures we read all the bytes
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
    fn load(reader: &mut impl Read) -> Network {
        let mut magic = [0u8; MAGIC.len()];
        reader.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        let w1 = Network::read_i16(reader, INPUT * HL1);
        let b1 = Network::read_i16(reader, HL1);

        let w2 = Network::read_i16(reader, HL1 * 2 * HL2);
        let b2 = Network::read_i16(reader, HL2);

        let w3 = Network::read_i16(reader, HL2 * HL3);
        let b3 = Network::read_i16(reader, HL3);

        let w4 = Network::read_i16(reader, HL3 * OUTPUT);
        let b4 = Network::read_i16(reader, OUTPUT);

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

    pub fn load_unquantized(path: &str) -> Network {
        let mut file = File::open(path).unwrap();
        let file_size = metadata(path).unwrap().len();

        let mut magic = [0u8; MAGIC.len()];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        let w1 = Network::read_f32(&mut file, INPUT * HL1);
        let w1 = Network::quantize_to_i16(&w1);

        let b1 = Network::read_f32(&mut file, HL1);
        let b1 = Network::quantize_to_i16(&b1);

        let w2 = Network::read_f32(&mut file, HL1 * 2 * HL2);
        let w2 = Network::quantize_to_i16(&w2);

        let b2 = Network::read_f32(&mut file, HL2);
        let b2 = Network::quantize_to_i16(&b2);

        let w3 = Network::read_f32(&mut file, HL2 * HL3);
        let w3 = Network::quantize_to_i16(&w3);

        let b3 = Network::read_f32(&mut file, HL3);
        let b3 = Network::quantize_to_i16(&b3);

        let w4 = Network::read_f32(&mut file, HL3 * OUTPUT);
        let w4 = Network::quantize_to_i16(&w4);

        let b4 = Network::read_f32(&mut file, OUTPUT);
        let b4 = Network::quantize_to_i16(&b4);

        let pos = file.stream_position().unwrap();
        assert_eq!(file_size, pos); // validating that we have reached the EOF

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

    fn quantize_to_i16(layer: &[f32]) -> Vec<i16> {
        let mut quantized: Vec<i16> = Vec::with_capacity(layer.len());

        for &val in layer {
            quantized.push((val * Q).round() as i16);
        }

        quantized
    }

    pub fn eval_hkp_with_acc(&self, buf: &mut EvalBuf, acc: &[i16]) -> i16 {
        buf.fc1.copy_from_slice(&acc);
        Network::hard_tanh(0 as i16, 1 * Q as i16, &mut buf.fc1);

        Network::process_layer(&buf.fc1, &mut buf.fc2, &self.w2, &self.b2, true);
        Network::hard_tanh(0 as i16, 1 * Q as i16, &mut buf.fc2);

        Network::process_layer(&buf.fc2, &mut buf.fc3, &self.w3, &self.b3, true);
        Network::hard_tanh(0 as i16, 1 * Q as i16, &mut buf.fc3);

        Network::process_layer(&buf.fc3, &mut buf.fc4, &self.w4, &self.b4, true);

        let y = buf.fc4[0] as f32 / Q as f32;
        let y = y.clamp(-0.99999, 0.99999);

        (600.0 * y.atanh()) as i16
    }

    pub fn eval_hkp(&self, board: &Board) -> i32 {
        let mut acc = self.build_acc(board);
        Network::hard_tanh(0 as i16, 1 * Q as i16, &mut acc);

        let mut fc2 = vec![0; HL2];
        Network::process_layer(&acc, &mut fc2, &self.w2, &self.b2, true);
        Network::hard_tanh(0 as i16, 1 * Q as i16, &mut fc2);

        let mut fc3 = vec![0; HL3];
        Network::process_layer(&fc2, &mut fc3, &self.w3, &self.b3, true);
        Network::hard_tanh(0 as i16, 1 * Q as i16, &mut fc3);

        let mut fc4 = vec![0; OUTPUT];
        Network::process_layer(&fc3, &mut fc4, &self.w4, &self.b4, true);

        let y = fc4[0] as f32 / Q as f32;
        let y = y.clamp(-0.99999, 0.99999);

        (600.0 * y.atanh()) as i32
    }

    fn build_acc(&self, board: &Board) -> [i16; HL1 * 2] {
        let mut white_feat = [50000usize; 30];
        let mut black_feat = [50000usize; 30];

        let bb: &[u64] = board.get_bb();
        let (mut wk, mut bk) = (bb[5], bb[11]);

        let w_king_pos = pop_lsb(&mut wk).expect("There aint no white king present dawg");
        let b_king_pos = mirror(pop_lsb(&mut bk).expect("There aint no black king present dawg"));

        let mut feat_idx = 0usize;
        for (idx, &bb) in bb.iter().enumerate() {
            if idx == 5 || idx == 11 {
                // skipping the kings
                continue;
            }

            let mut piece_bb = bb;

            while let Some(sq) = pop_lsb(&mut piece_bb) {
                if idx < 5 {
                    // White pieces
                    white_feat[feat_idx] = get_hkp_feature_idx(w_king_pos, idx, sq);
                    black_feat[feat_idx] = get_hkp_feature_idx(b_king_pos, idx + 5, mirror(sq));
                } else {
                    // Black pieces
                    white_feat[feat_idx] = get_hkp_feature_idx(w_king_pos, idx - 1, sq);
                    black_feat[feat_idx] = get_hkp_feature_idx(b_king_pos, idx - 6, mirror(sq));
                }
                feat_idx += 1;
            }
        }

        let w_acc = self.fill_acc(&white_feat[..feat_idx]);
        let b_acc = self.fill_acc(&black_feat[..feat_idx]);

        let mut acc = [0; HL1 * 2];

        if board.side_to_move() == Color::White {
            acc[..HL1].copy_from_slice(&w_acc);
            acc[HL1..].copy_from_slice(&b_acc);
        } else {
            acc[..HL1].copy_from_slice(&b_acc);
            acc[HL1..].copy_from_slice(&w_acc);
        };

        acc
    }

    // fn fill_acc(&self, feature: &[usize]) -> Vec<i16> {
    //     // Start the accumulator pre-loaded with the biases
    //     let mut acc = self.b1.clone();
    //
    //     for &act_feat in feature {
    //         let offset = act_feat * HL1;
    //
    //         assert!(offset + HL1 <= self.w1.len());
    //
    //         unsafe {
    //             for neuron_idx in 0..HL1 {
    //                 let w = *self.w1.get_unchecked(offset + neuron_idx);
    //                 *acc.get_unchecked_mut(neuron_idx) += w;
    //             }
    //         }
    //     }
    //
    //     acc
    // }

    // fn process_layer(
    //     inp_layer: &[i16],
    //     out_layer: &mut [i16],
    //     weight: &[i16],
    //     bias: &[i16],
    //     to_quantize: bool,
    // ) {
    //     let input_len = inp_layer.len();
    //     let out_len = bias.len();
    //
    //     // ensuring our slices are large enough to prevent UB
    //     assert!(out_layer.len() >= out_len);
    //     assert!(weight.len() >= input_len * out_len);
    //
    //     for neuron_idx in 0..out_len {
    //         let mut dot: i32 = 0;
    //         let w_offset = neuron_idx * input_len;
    //
    //         // Bypassing the bound checks for each array access
    //         unsafe {
    //             for i in 0..input_len {
    //                 let inp = *inp_layer.get_unchecked(i) as i32;
    //                 let w = *weight.get_unchecked(w_offset + i) as i32;
    //                 dot += inp * w;
    //             }
    //
    //             let b = *bias.get_unchecked(neuron_idx) as i32;
    //             let val = b + if to_quantize {
    //                 (dot + (1 << (QP - 1))) >> QP
    //             } else {
    //                 dot
    //             };
    //
    //             *out_layer.get_unchecked_mut(neuron_idx) = val as i16;
    //         }
    //     }
    // }
    //
    // fn hard_tanh(min: i16, max: i16, layer: &mut [i16]) {
    //     let len = layer.len();
    //     let mut idx = 0;
    //
    //     unsafe {
    //         while idx < len {
    //             let val = (*layer.get_unchecked(idx)).clamp(min, max);
    //             *layer.get_unchecked_mut(idx) = val;
    //             idx += 1;
    //         }
    //     }
    // }

    fn read_f32(file: &mut File, size: usize) -> Vec<f32> {
        let mut bytes = vec![0u8; size * 4];
        file.read_exact(&mut bytes).unwrap();

        let mut out = Vec::with_capacity(size);

        for chunk in bytes.chunks_exact(4) {
            out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        out
    }

    fn read_i16(file: &mut impl Read, size: usize) -> Vec<i16> {
        let mut bytes = vec![0u8; size * 2];
        file.read_exact(&mut bytes).unwrap();

        let mut out = Vec::with_capacity(size);

        for chunk in bytes.chunks_exact(2) {
            out.push(i16::from_le_bytes([chunk[0], chunk[1]]));
        }

        out
    }

    #[allow(dead_code)]
    fn read_i32(file: &mut impl Read, size: usize) -> Vec<i32> {
        let mut bytes = vec![0u8; size * 4];
        file.read_exact(&mut bytes).unwrap();

        let mut out = Vec::with_capacity(size);

        for chunk in bytes.chunks_exact(4) {
            out.push(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }

        out
    }
}

// SIMD instructions
impl Network {
    pub fn process_layer(
        inp_layer: &[i16],
        out_layer: &mut [i16],
        weight: &[i16],
        bias: &[i16],
        to_quantize: bool,
    ) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe {
                    Self::process_layer_avx2(inp_layer, out_layer, weight, bias, to_quantize)
                };
            }
        }

        Self::process_layer_scalar(inp_layer, out_layer, weight, bias, to_quantize);
    }

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

                *out_layer.get_unchecked_mut(neuron_idx) = val as i16;
            }
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
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

        for neuron_idx in 0..out_len {
            let w_offset = neuron_idx * input_len;
            let mut i = 0;

            unsafe {
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

                *out_layer.get_unchecked_mut(neuron_idx) = val as i16;
            }
        }
    }

    pub fn hard_tanh(min: i16, max: i16, layer: &mut [i16]) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { Self::hard_tanh_avx2(min, max, layer) };
            }
        }

        Self::hard_tanh_scalar(min, max, layer);
    }

    fn hard_tanh_scalar(min: i16, max: i16, layer: &mut [i16]) {
        let len = layer.len();
        let mut idx = 0;

        while idx < len {
            unsafe {
                let val = (*layer.get_unchecked(idx)).clamp(min, max);
                *layer.get_unchecked_mut(idx) = val;
            }
            idx += 1;
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn hard_tanh_avx2(min: i16, max: i16, layer: &mut [i16]) {
        let len = layer.len();
        let mut i = 0;

        unsafe {
            let v_min = _mm256_set1_epi16(min);
            let v_max = _mm256_set1_epi16(max);

            while i + 16 <= len {
                let ptr = layer.as_mut_ptr().add(i);

                let mut v = _mm256_loadu_si256(ptr as *const __m256i);
                v = _mm256_max_epi16(v, v_min);
                v = _mm256_min_epi16(v, v_max);

                _mm256_storeu_si256(ptr as *mut __m256i, v);
                i += 16;
            }

            while i < len {
                let val = (*layer.get_unchecked(i)).clamp(min, max);
                *layer.get_unchecked_mut(i) = val;
                i += 1;
            }
        }
    }

    pub fn fill_acc(&self, feature: &[usize]) -> Vec<i16> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self.fill_acc_avx2(feature) };
            }
        }

        self.fill_acc_scalar(feature)
    }

    fn fill_acc_scalar(&self, feature: &[usize]) -> Vec<i16> {
        let mut acc = self.b1.clone();

        for &act_feat in feature {
            let offset = act_feat * HL1;

            unsafe {
                for neuron_idx in 0..HL1 {
                    let w = *self.w1.get_unchecked(offset + neuron_idx);
                    *acc.get_unchecked_mut(neuron_idx) += w;
                }
            }
        }

        acc
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn fill_acc_avx2(&self, feature: &[usize]) -> Vec<i16> {
        let mut acc = self.b1.clone();

        for &act_feat in feature {
            let offset = act_feat * HL1;
            let mut i = 0;

            unsafe {
                while i + 16 <= HL1 {
                    let w_256 =
                        _mm256_loadu_si256(self.w1.as_ptr().add(offset + i) as *const __m256i);

                    let acc_ptr = acc.as_mut_ptr().add(i);
                    let acc_256 = _mm256_loadu_si256(acc_ptr as *const __m256i);

                    let sum = _mm256_add_epi16(acc_256, w_256);

                    _mm256_storeu_si256(acc_ptr as *mut __m256i, sum);

                    i += 16;
                }

                while i < HL1 {
                    let w = *self.w1.get_unchecked(offset + i);
                    *acc.get_unchecked_mut(i) += w;
                    i += 1;
                }
            }
        }

        acc
    }
}

fn get_hkp_feature_idx(king_pos: usize, piece_idx: usize, pos: usize) -> usize {
    king_pos * 640 + piece_idx * 64 + pos
}

impl Engine {
    pub fn setup_accumulator(&mut self) {
        if let Some(nn) = NETWORK.get() {
            let rebuild = nn.build_acc(&self.board);
            if self.board.side_to_move() == Color::White {
                self.accumulators[0][WHITE].copy_from_slice(&rebuild[..HL1]);
                self.accumulators[0][BLACK].copy_from_slice(&rebuild[HL1..]);
            } else {
                self.accumulators[0][BLACK].copy_from_slice(&rebuild[..HL1]);
                self.accumulators[0][WHITE].copy_from_slice(&rebuild[HL1..]);
            };
        }
    }

    pub fn update_nnue(&mut self, mv: &Move, undo: &Undo, ply: usize) {
        let Some(nn) = NETWORK.get() else {
            return;
        };

        // self.accumulators[ply + 1] = nn.build_acc(&self.board);
        self.accumulators[ply + 1] = self.accumulators[ply];

        let acc = &mut self.accumulators[ply + 1];

        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        // board is already after make_move()
        let moved_piece = self.board.piece_on(to);
        if Piece::get_type(moved_piece) == Piece::KING {
            let rebuild = nn.build_acc(&self.board);
            if self.board.side_to_move() == Color::White {
                acc[WHITE].copy_from_slice(&rebuild[..HL1]);
                acc[BLACK].copy_from_slice(&rebuild[HL1..]);
            } else {
                acc[BLACK].copy_from_slice(&rebuild[..HL1]);
                acc[WHITE].copy_from_slice(&rebuild[HL1..]);
            };
            return;
        }
        let side = Piece::get_color(moved_piece);

        let (w_king, b_king) = (
            self.board.bb(Piece::KING | Piece::WHITE).trailing_zeros() as usize,
            self.board.bb(Piece::KING | Piece::BLACK).trailing_zeros() as usize,
        );

        let mut w_removed = [0usize; 5];
        let mut w_added = [0usize; 5];

        let mut b_removed = [0usize; 5];
        let mut b_added = [0usize; 5];

        let mut r_cnt = 0;
        let mut a_cnt = 0;

        //
        // moved piece
        //

        if flag.is_promo() {
            w_removed[r_cnt] =
                get_hkp_feature_idx(w_king, Piece::to_hkp_idx(Piece::PAWN | side), from);
            b_removed[r_cnt] = get_hkp_feature_idx(
                mirror(b_king),
                (Piece::to_hkp_idx(Piece::PAWN | side) + 5) % 10,
                mirror(from),
            );
        } else {
            w_removed[r_cnt] = get_hkp_feature_idx(w_king, Piece::to_hkp_idx(moved_piece), from);
            b_removed[r_cnt] = get_hkp_feature_idx(
                mirror(b_king),
                (Piece::to_hkp_idx(moved_piece) + 5) % 10,
                mirror(from),
            );
        };
        r_cnt += 1;

        w_added[a_cnt] = get_hkp_feature_idx(w_king, Piece::to_hkp_idx(moved_piece), to);
        b_added[a_cnt] = get_hkp_feature_idx(
            mirror(b_king),
            (Piece::to_hkp_idx(moved_piece) + 5) % 10,
            mirror(to),
        );
        a_cnt += 1;

        //
        // captures
        //

        if flag.is_capture() {
            let cap_sq = if flag == MoveFlag::EN_PASSANT {
                if side == Piece::WHITE { to - 8 } else { to + 8 }
            } else {
                to
            };

            w_removed[r_cnt] =
                get_hkp_feature_idx(w_king, Piece::to_hkp_idx(undo.captured), cap_sq);
            b_removed[r_cnt] = get_hkp_feature_idx(
                mirror(b_king),
                (Piece::to_hkp_idx(undo.captured) + 5) % 10,
                mirror(cap_sq),
            );
            r_cnt += 1;
        }

        //
        // incremental accumulator update
        //

        // Added features
        for idx in 0..a_cnt {
            let (w_act, b_act) = (w_added[idx], b_added[idx]);

            for neuron in 0..HL1 {
                acc[WHITE][neuron] += nn.w1[w_act * (HL1) + neuron];
                acc[BLACK][neuron] += nn.w1[b_act * (HL1) + neuron];
            }
        }

        // Removed features
        for idx in 0..r_cnt {
            let (w_act, b_act) = (w_removed[idx], b_removed[idx]);

            for neuron in 0..HL1 {
                acc[WHITE][neuron] -= nn.w1[w_act * (HL1) + neuron];
                acc[BLACK][neuron] -= nn.w1[b_act * (HL1) + neuron];
            }
        }
    }

    pub fn update_nnue_null_move(&mut self, ply: usize) {
        self.accumulators[ply + 1] = self.accumulators[ply];
        return;
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        evaluation::init_pesto_table,
        magics::init_magics,
        network::{NN_DATA, Network, QP},
        zobrist::init_zobrist,
    };

    #[test]
    fn nn_check() {
        init_pesto_table();
        init_magics();

        init_zobrist();

        let mut cursor = Cursor::new(NN_DATA);
        let nn = Network::load(&mut cursor);

        println!("Quantized to 2 ^ {QP}");

        println!("Min w1: {}", nn.w1.iter().min().unwrap());
        println!("Max w1: {}", nn.w1.iter().max().unwrap());

        println!("Min b1: {}", nn.b1.iter().min().unwrap());
        println!("Max b1: {}", nn.b1.iter().max().unwrap());

        println!("Min w2: {}", nn.w2.iter().min().unwrap());
        println!("Max w2: {}", nn.w2.iter().max().unwrap());

        println!("Min b2: {}", nn.b2.iter().min().unwrap());
        println!("Max b2: {}", nn.b2.iter().max().unwrap());

        println!("Min w3: {}", nn.w3.iter().min().unwrap());
        println!("Max w3: {}", nn.w3.iter().max().unwrap());

        println!("Min b3: {}", nn.b3.iter().min().unwrap());
        println!("Max b3: {}", nn.b3.iter().max().unwrap());

        println!("Min w4: {}", nn.w4.iter().min().unwrap());
        println!("Max w4: {}", nn.w4.iter().max().unwrap());

        println!("Min b4: {}", nn.b4.iter().min().unwrap());
        println!("Max b4: {}", nn.b4.iter().max().unwrap());

        println!("Min i8 {}", i8::MIN);
        println!("Max i8 {}", i8::MAX);

        println!("Min i16 {}", i16::MIN);
        println!("Max i16 {}", i16::MAX);

        println!("Min i32 {}", i32::MIN);
        println!("Max i32 {}", i32::MAX);
    }
}
