use roxie::{evaluation::init_pesto_table, magics::init_magics, uci::UCI, zobrist::init_zobrist};

fn init_all() {
    init_zobrist();
    init_pesto_table();
    init_magics();
}

fn main() {
    init_all();
    let mut uci = UCI::new();
    uci.uci_loop();
}

#[cfg(test)]
mod tests {
    use roxie::{board::Board, engine::Engine, perft::perft, search::SearchLimits};
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
