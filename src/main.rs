use roxie::{
    evaluation::init_pesto_table, magics::init_magics, network::init_nn, uci::UCI,
    zobrist::init_zobrist,
};

fn init_all() {
    init_zobrist();
    init_pesto_table();
    init_magics();
    init_nn(false);
}

fn main() {
    init_all();
    let mut uci = UCI::new();
    uci.uci_loop();
}

#[cfg(test)]
mod tests {
    use roxie::{
        board::Board, engine::Engine, network::Network, perft::perft,
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
    fn nn_load() {
        let _ = Network::load("roxie_v1.nn");
    }

    #[test]
    fn nn_eval() {
        init_all();
        let fens = [
            "r1bqkbnr/ppp3pp/2np4/4pp2/4P3/2NP1N1P/PPP2PP1/R1BQKB1R b KQkq - 1 5",
            "r2qkbnr/p1pp1P2/1p2p3/6p1/2Pn3p/2N2Q1P/PP3PP1/R1B1KB1R b KQkq - 0 11",
            "8/8/1K2pR2/4P3/4kP2/8/8/8 w - - 5 59",
            "8/8/1p4R1/6b1/1PP3kp/P5p1/4K1B1/8 b - - 0 43",
            "rn1q1bnr/pp2pk1p/3pb1p1/3p4/4P3/5N2/PPP2PPP/RNB1KB1R w KQ - 0 7",
        ];
        let nn = Network::load("roxie_v1.nn");

        let mut time = 0;
        for fen in fens {
            let board = Board::load_fen(fen);
            let start = Instant::now();
            let eval = nn.eval(&board);
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
