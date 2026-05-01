use roxie::{uci::uci_loop, zobrist::init_zobrist};

fn main() {
    init_zobrist();
    uci_loop();
}

#[cfg(test)]
mod tests {
    use roxie::{
        board::{Board},
        perft::perft,
        search::find_best_move,
        zobrist::init_zobrist,
    };
    const FEN: &str = "r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1 ";

    static DEPTH_1: u64 = 6;
    static DEPTH_2: u64 = 264;
    static DEPTH_3: u64 = 9467;
    static DEPTH_4: u64 = 422333;
    static DEPTH_5: u64 = 15833292;

    #[test]
    fn best_move() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);
        _ = find_best_move(&mut board, 5);
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

    #[test]
    fn perft_5() {
        init_zobrist();
        let mut board = Board::load_fen(FEN);
        assert_eq!(perft(&mut board, 5), DEPTH_5);
    }
}
