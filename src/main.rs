fn main() {
    println!("Normal main entry point");
}

#[cfg(test)]
mod tests {
    use roxie::{board::Board, perft::perft};

    #[test]
    fn perft_1() {
        let mut board = Board::start_pos();
        assert_eq!(perft(&mut board, 1), 20);
    }

    #[test]
    fn perft_2() {
        let mut board = Board::start_pos();
        assert_eq!(perft(&mut board, 2), 400);
    }

    #[test]
    fn perft_3() {
        let mut board = Board::start_pos();
        assert_eq!(perft(&mut board, 3), 8902);
    }

    #[test]
    fn perft_4() {
        let mut board = Board::start_pos();
        assert_eq!(perft(&mut board, 4), 197_281);
    }

    #[test]
    fn perft_5() {
        let mut board = Board::start_pos();
        assert_eq!(perft(&mut board, 5), 4_865_609);
    }
}
