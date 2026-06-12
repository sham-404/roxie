use std::{fs::{File, metadata}, io::{Read, Seek}};

const INPUT: usize = 769;
const HL1: usize = 1024;
const HL2: usize = 32;
const OUTPUT: usize = 1;
const MAGIC: &[u8; 7] = b"ROXIE_F";

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
