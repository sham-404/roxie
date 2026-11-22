#![cfg_attr(rust_analyzer, allow(rust_analyzer::inactive_code))]

use crate::{
    board::init_ray_tables, evaluation::init_pesto_table, magics::init_magics, network::init_nn,
    search::init_lmr_table, uci::UCI, zobrist::init_zobrist,
};

mod board;
mod r#const;
mod engine;
mod evaluation;
mod items;
mod magics;
mod move_pick;
mod network;
mod params;
mod perft;
mod search;
mod square;
mod syzygy;
mod tt;
mod uci;
mod zobrist;

fn init_all() {
    init_zobrist();
    init_pesto_table();
    init_magics();
    init_lmr_table();
    init_nn();
    init_ray_tables();
}

fn main() {
    init_all();
    let mut uci = UCI::new();
    uci.uci_loop();
}

#[cfg(test)]
mod tests;
