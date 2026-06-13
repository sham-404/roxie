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
pub const HL1: usize = 512;
const HL2: usize = 256;
const OUTPUT: usize = 1;
const MAGIC: &[u8; 7] = b"ROXIE_F";

pub static NETWORK: OnceLock<Network> = OnceLock::new();

pub fn init_nn(is_needed: bool) {
    if !is_needed {
        return;
    }
    NETWORK.get_or_init(|| Network::load("roxie_v1.nn"));
}

pub struct Network {
    w1: Vec<f32>,
    b1: Vec<f32>,
    w2: Vec<f32>,
    b2: Vec<f32>,
    w3: Vec<f32>,
    b3: Vec<f32>,
}

impl Network {
    pub fn load(path: &str) -> Network {
        let mut file = File::open(path).unwrap();
        let file_size = metadata(path).unwrap().len();

        let mut magic = [0u8; MAGIC.len()];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, MAGIC);

        let w1 = Network::read_f32(&mut file, INPUT * HL1);
        let b1 = Network::read_f32(&mut file, HL1);
        let w2 = Network::read_f32(&mut file, HL1 * HL2);
        let b2 = Network::read_f32(&mut file, HL2);
        let w3 = Network::read_f32(&mut file, HL2 * OUTPUT);
        let b3 = Network::read_f32(&mut file, OUTPUT);

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

    pub fn eval(&self, board: &Board) -> i32 {
        let mut feature: Vec<f32> = vec![0.0; INPUT];

        for (idx, &bb) in board.get_bb().iter().enumerate() {
            let mut cur_bb = bb;

            while let Some(sq) = pop_lsb(&mut cur_bb) {
                let feat_idx = sq * 12 + idx;
                feature[feat_idx] = 1.0;
            }
        }

        if board.side_to_move() == Color::White {
            feature[INPUT - 1] = 1.0;
        }

        let normalized_cp = self.forward(&feature).clamp(0.00001, 0.99999);
        let cp = (700.0 * (normalized_cp / (1.0 - normalized_cp)).ln()) as i32;

        cp
    }

    pub fn evaluate_with_acc(&self, acc: &[f32]) -> i32 {
        let fc1 = Network::relu_layer(acc);

        let fc2 = Network::process_layer(&fc1, &self.w2, &self.b2);
        let fc2 = Network::relu_layer(&fc2);

        let fc3 = Network::process_layer(&fc2, &self.w3, &self.b3);
        let fc3 = Network::sigmoid_layer(&fc3);

        let p = fc3[0];
        (700.0 * (p / (1.0 - p)).ln()) as i32
    }

    pub fn forward(&self, feature: &[f32]) -> f32 {
        let fc1 = Network::process_layer(feature, &self.w1, &self.b1);
        let fc1 = Network::relu_layer(&fc1);

        let fc2 = Network::process_layer(&fc1, &self.w2, &self.b2);
        let fc2 = Network::relu_layer(&fc2);

        let fc3 = Network::process_layer(&fc2, &self.w3, &self.b3);
        let fc3 = Network::sigmoid_layer(&fc3);
        fc3[0]
    }

    fn process_layer(layer: &[f32], weight: &[f32], bias: &[f32]) -> Vec<f32> {
        let mut result = Vec::with_capacity(bias.len());
        let input_len = layer.len();

        for neuron_idx in 0..bias.len() {
            let mut val = bias[neuron_idx];

            for i in 0..input_len {
                val += layer[i] * weight[i + neuron_idx * input_len];
            }

            // activation
            result.push(val);
        }

        result
    }

    fn sigmoid_layer(layer: &[f32]) -> Vec<f32> {
        let mut res = Vec::with_capacity(layer.len());

        for i in 0..layer.len() {
            res.push(Network::sigmoid(layer[i]));
        }

        res
    }

    fn relu_layer(layer: &[f32]) -> Vec<f32> {
        let mut res = Vec::with_capacity(layer.len());

        for i in 0..layer.len() {
            res.push(Network::relu(layer[i]));
        }

        res
    }

    fn sigmoid(val: f32) -> f32 {
        1.0 / (1.0 + (-val).exp())
    }

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
    pub fn setup_accumulator(&self) -> [f32; HL1] {
        if let Some(nn) = NETWORK.get() {
            let mut feature: Vec<f32> = vec![0.0; INPUT];

            for (idx, &bb) in self.board.get_bb().iter().enumerate() {
                let mut cur_bb = bb;

                while let Some(sq) = pop_lsb(&mut cur_bb) {
                    let feat_idx = sq * 12 + idx;
                    feature[feat_idx] = 1.0;
                }
            }

            if self.board.side_to_move() == Color::White {
                feature[INPUT - 1] = 1.0;
            }

            let acc = Network::process_layer(&feature, &nn.w1, &nn.b1);
            acc.try_into().unwrap()
        } else {
            [0.0; HL1]
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
            let mut delta = 0.0;

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

            acc[neuron] += delta;
        }
    }

    pub fn update_nnue_null_move(&mut self, ply: usize) {
        let Some(nn) = NETWORK.get() else {
            return;
        };

        let acc = &mut self.accumulators[ply + 1];

        for neuron in 0..HL1 {
            let mut delta = 0.0;

            // side-to-move feature
            // feature[768] = 1 when White to move

            let stm_weight = nn.w1[(INPUT - 1) + neuron * INPUT];

            if self.board.side_to_move() == Color::White {
                delta += stm_weight;
            } else {
                delta -= stm_weight;
            }

            acc[neuron] += delta;
        }
    }
}
