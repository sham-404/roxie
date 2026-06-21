use roxie::{
    evaluation::init_pesto_table, magics::init_magics, network::init_nn, uci::UCI,
    zobrist::init_zobrist,
};

fn init_all() {
    init_zobrist();
    init_pesto_table();
    init_magics();
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
        board::Board,
        engine::Engine,
        network::{NETWORK, Network},
        perft::perft,
        search::SearchLimits,
    };
    use std::time::Instant;

    use crate::init_all;

    #[test]
    fn analysis() {
        init_all();

        let mut engine: Engine = Engine::new();
        // startpos perft evaluation
        {
            engine.board = Board::start_pos();

            let start = Instant::now();
            let nodes = perft(&mut engine.board, 5);
            let duration = start.elapsed();

            let secs = duration.as_secs_f64();
            let nps = (nodes as f64 / secs) as u64;

            println!(
                "perft depth 5 (startpos): nodes={} time={:.5}s nps={}",
                nodes, secs, nps
            );
            assert_eq!(nodes, 4_865_609);
        }

        // kiwipete perft evaluation
        {
            engine.board = Board::load_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            );

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

        // kiwipete search analysis
        {
            engine.board = Board::load_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            );
            let start = Instant::now();
            let data = engine.search_ids(&SearchLimits::with_depth(6), |_| {});
            let duration = start.elapsed();

            let secs = duration.as_secs_f64();
            let nps = (data.nodes as f64 / secs) as u64;

            println!(
                "search depth 6 (kiwipete): nodes={} time={:.5}s nps={}",
                data.nodes, secs, nps
            );
        }
    }

    #[test]
    fn search() {
        init_all();
        let mut engine = Engine::new();
        engine.board =
            Board::load_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ");

        let depth = 7;
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
    fn nn_qver() {
        use roxie::network::Q;
        use std::{
            fs::File,
            io::{BufRead, BufReader},
        };

        init_all();

        let nn = NETWORK.get().unwrap();

        let file = File::open("nn_qver.csv").unwrap();
        let reader = BufReader::new(file);

        let mut total_err: i64 = 0;
        let mut max_err = 0;
        let mut count = 0;

        for (idx, line) in reader.lines().enumerate() {
            let line = line.unwrap();

            if idx == 0 {
                continue;
            }

            let (fen, expected) = line.rsplit_once(',').unwrap();

            let expected: i32 = expected.parse().unwrap();

            let board = Board::load_fen(fen);

            let quant_eval = nn.eval_hkp(&board);

            let err = (expected - quant_eval).abs();

            total_err += err as i64;
            max_err = max_err.max(err);

            count += 1;
        }

        println!("positions: {}", count);
        println!("Q = {}", Q);
        println!("mae: {:.2}", total_err as f64 / count as f64);
        println!("max error: {}", max_err);
    }

    #[test]
    fn nn_load() {
        let _ = Network::load("blaze_v2.nnue");
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

    fn qperft(board: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }

        let in_check = board.in_check();

        let moves = if in_check {
            board.gen_moves()
        } else {
            board.gen_cap_moves()
        };

        let mut nodes = 0;

        for mv in moves.as_slice() {
            let undo = board.make_move(mv);

            nodes += qperft(board, depth - 1);

            board.unmake_move(mv, &undo);
        }

        nodes
    }

    #[test]
    fn qperft_test() {
        init_all();
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

        let mut board = Board::load_fen(fen);

        for depth in 1..=5 {
            let nodes = qperft(&mut board, depth);
            println!("depth {}: {}", depth, nodes);
        }
    }
}
