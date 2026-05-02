use roxie::{uci::uci_loop, zobrist::init_zobrist};

fn main() {
    init_zobrist();
    uci_loop();
}

#[cfg(test)]
mod tests {
    use roxie::{board::Board, perft::perft, search::find_best_move, zobrist::init_zobrist};
    const FEN: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ";

    static DEPTH_1: u64 = 48;
    static DEPTH_2: u64 = 2039;
    static DEPTH_3: u64 = 97862;
    static DEPTH_4: u64 = 4085603;
    static DEPTH_5: u64 = 193690690;
    static DEPTH_6: u64 = 8031647685;

    #[test]
    fn best_move() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);

        let start = Instant::now();

        let depth = 5;
        let (_, nodes) = find_best_move(&mut board, depth);

        let duration = start.elapsed();
        let secs = duration.as_secs_f64();
        let nps = (nodes as f64 / secs) as u64;

        println!("search({}): nodes={} time={:.3}s nps={}",depth, nodes, secs, nps);
    }

    #[test]
    fn perft_1() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);
        assert_eq!(perft(&mut board, 1), DEPTH_1);
    }

    #[test]
    fn perft_2() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);
        assert_eq!(perft(&mut board, 2), DEPTH_2);
    }

    #[test]
    fn perft_3() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);
        assert_eq!(perft(&mut board, 3), DEPTH_3);
    }

    #[test]
    fn perft_4() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);
        assert_eq!(perft(&mut board, 4), DEPTH_4);
    }

    use std::time::Instant;

    #[test]
    fn perft_5() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);

        let start = Instant::now();

        let nodes = perft(&mut board, 5);

        let duration = start.elapsed();
        let secs = duration.as_secs_f64();
        let nps = (nodes as f64 / secs) as u64;

        println!("perft(5): nodes={} time={:.3}s nps={}", nodes, secs, nps);

        assert_eq!(nodes, DEPTH_5);
    }

    #[test]
    fn perft_6() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);

        let start = Instant::now();

        let nodes = perft(&mut board, 6);

        let duration = start.elapsed();
        let secs = duration.as_secs_f64();
        let nps = (nodes as f64 / secs) as u64;

        println!("perft(6): nodes={} time={:.3}s nps={}", nodes, secs, nps);

        assert_eq!(nodes, DEPTH_6);
    }

    #[test]
    fn start_pos() {
        init_zobrist();
        let mut board = Board::start_pos();

        let start = Instant::now();

        let nodes = perft(&mut board, 5);

        let duration = start.elapsed();
        let secs = duration.as_secs_f64();
        let nps = (nodes as f64 / secs) as u64;

        println!("perft(5): nodes={} time={:.3}s nps={}", nodes, secs, nps);
    }
}
