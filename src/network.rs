use std::{
    fs::{File, metadata},
    io::{Read, Seek},
    sync::OnceLock,
};

use crate::{
    board::{Board, pop_lsb},
    engine::Engine,
    items::{Color, Move, MoveFlag, Piece, PieceInfo, Undo},
};

const INPUT: usize = 769;
pub const HL1: usize = 1024;
const HL2: usize = 64;
const OUTPUT: usize = 1;
const MAGIC: &[u8; 8] = b"ROXIE_V2";
const NN_PATH: &'static str = "roxie_v2.nn";
const QP: i32 = 9;
pub const Q: f32 = (1 << QP) as f32; // 2 ^ 6

pub static NETWORK: OnceLock<Network> = OnceLock::new();

pub fn init_nn(is_needed: bool) {
    if !is_needed {
        return;
    }
    NETWORK.get_or_init(|| Network::load(NN_PATH));
}

pub struct Network {
    w1: Vec<i16>,
    b1: Vec<i32>,
    w2: Vec<i16>,
    b2: Vec<i32>,
    w3: Vec<i16>,
    b3: Vec<i32>,
}

impl Network {
    pub fn load(path: &str) -> Network {
        let mut file = File::open(path).unwrap();
        let file_size = metadata(path).unwrap().len();

        let mut magic = [0u8; MAGIC.len()];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        let w1 = Network::read_f32(&mut file, INPUT * HL1);
        let w1 = Network::quantize_to_i16(&w1);

        let b1 = Network::read_f32(&mut file, HL1);
        let b1 = Network::quantize_to_i32(&b1);

        let w2 = Network::read_f32(&mut file, HL1 * HL2);
        let w2 = Network::quantize_to_i16(&w2);

        let b2 = Network::read_f32(&mut file, HL2);
        let b2 = Network::quantize_to_i32(&b2);

        let w3 = Network::read_f32(&mut file, HL2 * OUTPUT);
        let w3 = Network::quantize_to_i16(&w3);

        let b3 = Network::read_f32(&mut file, OUTPUT);
        let b3 = Network::quantize_to_i32(&b3);

        let pos = file.stream_position().unwrap();
        assert_eq!(file_size, pos); // validating that we have reached the EOF

        Network {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
        }
    }

    fn quantize_to_i16(layer: &[f32]) -> Vec<i16> {
        let mut quantized: Vec<i16> = Vec::with_capacity(layer.len());

        for &val in layer {
            quantized.push((val * Q).round() as i16);
        }

        quantized
    }

    fn quantize_to_i32(layer: &[f32]) -> Vec<i32> {
        let mut quantized: Vec<i32> = Vec::with_capacity(layer.len());

        for &val in layer {
            quantized.push((val * Q).round() as i32);
        }

        quantized
    }

    pub fn eval(&self, board: &Board) -> i32 {
        let mut feature: Vec<i32> = vec![0; INPUT];

        for (idx, &bb) in board.get_bb().iter().enumerate() {
            let mut cur_bb = bb;

            while let Some(sq) = pop_lsb(&mut cur_bb) {
                let feat_idx = sq * 12 + idx;
                feature[feat_idx] = 1;
            }
        }

        if board.side_to_move() == Color::White {
            feature[INPUT - 1] = 1;
        }

        let normalized_cp = self.forward(&feature);

        // let cp = (700.0 * (normalized_cp / (1.0 - normalized_cp)).ln()) as i32;
        let cp = (normalized_cp * 400) as i32;

        cp >> QP
    }

    pub fn evaluate_with_acc(&self, acc: &[i32]) -> i32 {
        let fc1 = Network::hard_tanh(0 * Q as i32, 1 * Q as i32, acc);

        let fc2 = Network::process_layer(&fc1, &self.w2, &self.b2, true);
        let fc2 = Network::hard_tanh(0 * Q as i32, 1 * Q as i32, &fc2);

        let fc3 = Network::process_layer(&fc2, &self.w3, &self.b3, true);

        let p = fc3[0];

        // let cp = (700.0 * (p / (1.0 - p)).ln()) as i32;
        (p * 400) >> QP as i32
    }

    pub fn forward(&self, feature: &[i32]) -> i32 {
        let fc1 = Network::process_layer(feature, &self.w1, &self.b1, false);
        let fc1 = Network::hard_tanh(0 * Q as i32, 1 * Q as i32, &fc1);

        let fc2 = Network::process_layer(&fc1, &self.w2, &self.b2, true);
        let fc2 = Network::hard_tanh(0 * Q as i32, 1 * Q as i32, &fc2);

        let fc3 = Network::process_layer(&fc2, &self.w3, &self.b3, true);
        fc3[0]
    }

    fn process_layer(layer: &[i32], weight: &[i16], bias: &[i32], to_quantize: bool) -> Vec<i32> {
        let q = if to_quantize { QP as i32 } else { 0 };

        let mut result = Vec::with_capacity(bias.len());
        let input_len = layer.len();

        for neuron_idx in 0..bias.len() {
            let mut dot = 0;

            for i in 0..input_len {
                dot += layer[i] * weight[i + neuron_idx * input_len] as i32;
            }

            let val = bias[neuron_idx]
                + if to_quantize {
                    (dot + (1 << (q - 1))) >> q
                } else {
                    dot
                };

            result.push(val);
        }

        result
    }

    #[allow(dead_code)]
    fn sigmoid_layer(layer: &[f32]) -> Vec<f32> {
        let mut res = Vec::with_capacity(layer.len());

        for i in 0..layer.len() {
            res.push(Network::sigmoid(layer[i]));
        }

        res
    }

    #[allow(dead_code)]
    fn relu_layer(layer: &[f32]) -> Vec<f32> {
        let mut res = Vec::with_capacity(layer.len());

        for i in 0..layer.len() {
            res.push(Network::relu(layer[i]));
        }

        res
    }

    #[allow(dead_code)]
    fn hard_tanh(min: i32, max: i32, layer: &[i32]) -> Vec<i32> {
        let mut res = Vec::with_capacity(layer.len());

        for &val in layer {
            res.push(val.clamp(min, max));
        }

        res
    }

    #[allow(dead_code)]
    fn sigmoid(val: f32) -> f32 {
        1.0 / (1.0 + (-val).exp())
    }

    #[allow(dead_code)]
    fn relu(val: f32) -> f32 {
        val.max(0.0)
    }

    fn read_f32(file: &mut File, size: usize) -> Vec<f32> {
        let mut buf = [0u8; 4];
        let mut vec_f32: Vec<f32> = Vec::with_capacity(size);

        for _ in 0..size {
            file.read_exact(&mut buf).unwrap();
            vec_f32.push(f32::from_le_bytes(buf));
        }

        vec_f32
    }
}

fn get_feature_idx(piece: PieceInfo, pos: usize) -> usize {
    let piece_idx = Piece::to_idx(piece);
    pos * 12 + piece_idx
}

impl Engine {
    pub fn setup_accumulator(&self) -> [i32; HL1] {
        if let Some(nn) = NETWORK.get() {
            let mut feature: Vec<i32> = vec![0; INPUT];

            for (idx, &bb) in self.board.get_bb().iter().enumerate() {
                let mut cur_bb = bb;

                while let Some(sq) = pop_lsb(&mut cur_bb) {
                    let feat_idx = sq * 12 + idx;
                    feature[feat_idx] = 1;
                }
            }

            if self.board.side_to_move() == Color::White {
                feature[INPUT - 1] = 1;
            }

            let acc = Network::process_layer(&feature, &nn.w1, &nn.b1, false);
            acc.try_into().unwrap()
        } else {
            [0; HL1]
        }
    }

    pub fn update_nnue(&mut self, mv: &Move, undo: &Undo, ply: usize) {
        let Some(nn) = NETWORK.get() else {
            return;
        };

        self.accumulators[ply + 1] = self.accumulators[ply];
        let acc = &mut self.accumulators[ply + 1];

        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        // board is already after make_move()
        let moved_piece = self.board.piece_on(to);
        let side = Piece::get_color(moved_piece);

        let mut removed = [0usize; 4];
        let mut added = [0usize; 3];

        let mut r_cnt = 0;
        let mut a_cnt = 0;

        //
        // moved piece
        //

        removed[r_cnt] = if flag.is_promo() {
            get_feature_idx(Piece::PAWN | side, from)
        } else {
            get_feature_idx(moved_piece, from)
        };
        r_cnt += 1;

        added[a_cnt] = get_feature_idx(moved_piece, to);
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

            removed[r_cnt] = get_feature_idx(undo.captured, cap_sq);
            r_cnt += 1;
        }

        //
        // castling rook movement
        //

        if flag.is_castle() {
            let (rook_from, rook_to) = match flag {
                MoveFlag::KING_CASTLE => {
                    if side == Piece::WHITE {
                        (63, 61)
                    } else {
                        (7, 5)
                    }
                }
                MoveFlag::QUEEN_CASTLE => {
                    if side == Piece::WHITE {
                        (56, 59)
                    } else {
                        (0, 3)
                    }
                }
                _ => unreachable!(),
            };

            let rook = Piece::ROOK | side;

            removed[r_cnt] = get_feature_idx(rook, rook_from);
            r_cnt += 1;

            added[a_cnt] = get_feature_idx(rook, rook_to);
            a_cnt += 1;
        }

        //
        // incremental accumulator update
        //

        for neuron in 0..HL1 {
            let mut delta = 0;

            for r in 0..r_cnt {
                delta -= nn.w1[removed[r] + neuron * INPUT];
            }

            for a in 0..a_cnt {
                delta += nn.w1[added[a] + neuron * INPUT];
            }

            //
            // side-to-move feature
            //
            // feature[768] = 1 when White to move
            //

            let stm_weight = nn.w1[(INPUT - 1) + neuron * INPUT];

            if self.board.side_to_move() == Color::White {
                delta += stm_weight;
            } else {
                delta -= stm_weight;
            }

            acc[neuron] += delta as i32;
        }
    }

    pub fn update_nnue_null_move(&mut self, ply: usize) {
        let Some(nn) = NETWORK.get() else {
            return;
        };

        let acc = &mut self.accumulators[ply + 1];

        for neuron in 0..HL1 {
            let mut delta = 0;

            // side-to-move feature
            // feature[768] = 1 when White to move

            let stm_weight = nn.w1[(INPUT - 1) + neuron * INPUT];

            if self.board.side_to_move() == Color::White {
                delta += stm_weight;
            } else {
                delta -= stm_weight;
            }

            acc[neuron] += delta as i32;
        }
    }
}
