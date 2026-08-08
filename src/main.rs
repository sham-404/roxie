use roxie::{
    evaluation::init_pesto_table, magics::init_magics, network::init_nn, search::init_lmr_table,
    uci::UCI, zobrist::init_zobrist,
};

fn init_all() {
    init_zobrist();
    init_pesto_table();
    init_magics();
    init_lmr_table();
    init_nn(true);
}

fn main() {
    init_all();
    let mut uci = UCI::new();
    uci.uci_loop();
}

#[cfg(test)]
mod tests {
    use roxie::{
        board::Board, engine::Engine, network::NETWORK, perft::perft, search::SearchLimits,
    };
    use std::time::Instant;

    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use crate::init_all;

    #[test]
    fn move_gen() {
        init_all();

        let mut engine: Engine = Engine::new();
        engine.board =
            Board::load_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ");

        let start = Instant::now();
        let nodes = perft(&mut engine.board, 5);
        let duration = start.elapsed();

        let secs = duration.as_secs_f64();
        let nps = (nodes as f64 / secs) as u64;

        println!(
            "perft depth 5 (kiwipete): nodes={} time={:.5}s nps={}",
            nodes, secs, nps
        );
        assert_eq!(nodes, 193_690_690);
    }

    #[test]
    fn search() {
        init_all();
        let mut engine = Engine::new();
        engine.board =
            Board::load_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ");

        let depth = 15;
        let start = Instant::now();
        let data = engine.search_ids(&SearchLimits::with_depth(depth), |_| {});
        let duration = start.elapsed();

        let secs = duration.as_secs_f64();
        let nps = (data.nodes as f64 / secs) as u64;

        println!(
            "search depth {} (kiwipete): nodes searched={} time={:.5}s nps={}",
            depth, data.nodes, secs, nps
        );
    }

    #[test]
    fn split_mv_gen() {
        init_all();

        let file = File::open("tests/sample_positions.csv").unwrap();
        let reader = BufReader::new(file);

        for (idx, line) in reader.lines().enumerate() {
            let line = line.unwrap();

            if idx == 0 {
                continue;
            }

            let (fen, _) = line.rsplit_once(',').unwrap();
            let mut board = Board::load_fen(fen);

            let mut total_moves = board.gen_moves();
            let mut quiet_moves = board.gen_quiet_moves();
            let mut tactic_moves = board.gen_tactical_moves();

            board.filter_illegal(&mut total_moves);
            board.filter_illegal(&mut quiet_moves);
            board.filter_illegal(&mut tactic_moves);

            assert_eq!(total_moves.len(), quiet_moves.len() + tactic_moves.len());
        }
    }

    #[test]
    fn nn_eval() {
        init_all();

        let fens = [
            "r2qkbr1/pb1nn3/1ppp3p/8/3P1p2/2PB1N1P/PPQN1PP1/2K1R2R w q - 2 15",
            "r2qkb2/pb1nn3/1ppp2rp/8/3P1p2/2P2N1P/PPQN1PP1/2K1R2R w q - 0 16",
            "r2qkbr1/pb1nn3/1ppp2Bp/8/3P1p2/2P2N1P/PPQN1PP1/2K1R2R b q - 3 15",
            "8/7p/R5p1/2p1pkP1/7P/P4PK1/1r6/3q4 w - - 6 46",
            "6k1/pp6/3p4/2p1p3/2P1P1q1/1P1P2pP/P5P1/5K2 w - - 0 31",
        ];
        let nn = NETWORK.get().unwrap();

        let mut time = 0;
        for fen in fens {
            let board = Board::load_fen(fen);
            let start = Instant::now();
            let eval = nn.eval_hkp(&board);
            let duration = start.elapsed();
            let elapsed = duration.as_nanos();

            time += elapsed;

            println!("fen: {}", fen);
            println!("roxie's nn eval: {}", eval);
            println!("eval duration: {}ns", elapsed);
            println!();
        }

        println!("Time taken: {}ns", time);
    }
}
