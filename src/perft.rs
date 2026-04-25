use crate::board::Board;

pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = board.gen_moves();

    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0;

    for mov in moves {
        let undo = board.make_move(&mov);
        nodes += perft(board, depth - 1);
        board.undo_move(&mov, &undo);
    }

    nodes
}
