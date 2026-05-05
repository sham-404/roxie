use roxie::{evaluation::init_pesto_table, uci::uci_loop, zobrist::init_zobrist};

fn main() {
    init_zobrist();
    init_pesto_table();
    uci_loop();
}

#[cfg(test)]
mod tests {
    use roxie::{
        board::Board,
        evaluation::init_pesto_table,
        perft::perft,
        search::{find_best_move, search_ids},
        transposition_table::TranspositionTable,
        zobrist::init_zobrist,
    };
    use std::time::Instant;

    #[test]
    fn analysis() {
        init_zobrist();
        init_pesto_table();

        let mut board: Board;
        // startpos perft evaluation
        {
            board = Board::start_pos();

            let start = Instant::now();
            let nodes = perft(&mut board, 5);
            let duration = start.elapsed();

            let secs = duration.as_secs_f64();
            let nps = (nodes as f64 / secs) as u64;

            println!(
                "pertf depth 5 (startpos): nodes={} time={:.5}s nps={}",
                nodes, secs, nps
            );
            assert_eq!(nodes, 4_865_609);
        }

        // kiwipete perft evaluation
        {
            board = Board::load_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            );

            let start = Instant::now();
            let nodes = perft(&mut board, 5);
            let duration = start.elapsed();

            let secs = duration.as_secs_f64();
            let nps = (nodes as f64 / secs) as u64;

            println!(
                "pertf depth 5 (kiwipete): nodes={} time={:.5}s nps={}",
                nodes, secs, nps
            );
            assert_eq!(nodes, 193_690_690);
        }

        // kiwipete search analysis
        {
            board = Board::load_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            );
            let mut tt = TranspositionTable::new(16);
            let start = Instant::now();
            let data = find_best_move(&mut board, 5, &mut tt);
            let duration = start.elapsed();

            let secs = duration.as_secs_f64();
            let nps = (data.nodes as f64 / secs) as u64;

            println!(
                "search depth 5 (kiwipete): nodes={} time={:.5}s nps={}",
                data.nodes, secs, nps
            );
        }
    }

    #[test]
    fn search() {
        init_zobrist();
        init_pesto_table();
        let mut board = Board::start_pos();
            // Board::load_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ");

        let depth = 7;
        let mut tt = TranspositionTable::new(16);
        let start = Instant::now();
        let data = search_ids(&mut board, depth, &mut tt);
        let duration = start.elapsed();

        let secs = duration.as_secs_f64();
        let nps = (data.nodes as f64 / secs) as u64;

        println!(
            "search depth {} (startpos): nodes searched={} time={:.5}s nps={}",
            depth, data.nodes, secs, nps
        );
    }
}
