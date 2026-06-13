use std::{
    fs::{File, metadata},
    io::{Read, Seek},
    sync::OnceLock,
};

use crate::{
    board::{Board, pop_lsb},
    items::Color,
};

const INPUT: usize = 769;
const HL1: usize = 512;
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
