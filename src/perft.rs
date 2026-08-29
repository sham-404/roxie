use crate::board::Board;

pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let move_list = board.gen_moves();
    let (checkers, pinned) = board.checkers_and_pinned();

    if depth == 1 {
        return move_list
            .as_slice()
            .iter()
            .filter(|&&mv| board.is_legal_fast(mv, checkers, pinned))
            .count() as u64;
    }

    let mut nodes = 0;

    for mov in move_list.as_slice() {
        if !board.is_legal_fast(*mov, checkers, pinned) {
            continue;
        }

        let undo = board.make_move(mov);
        nodes += perft(board, depth - 1);
        board.unmake_move(mov, &undo);
    }

    nodes
}

pub fn perft_divide(board: &mut Board, depth: u32) -> u64 {
    let move_list = board.gen_moves();
    let (checkers, pinned) = board.checkers_and_pinned();
    let mut total_nodes = 0;

    for mov in move_list.as_slice() {
        if !board.is_legal_fast(*mov, checkers, pinned) {
            continue;
        }

        let undo = board.make_move(&mov);

        let nodes = if depth > 1 {
            perft(board, depth - 1)
        } else {
            1
        };

        board.unmake_move(&mov, &undo);

        println!("{}: {}", mov.to_coord(), nodes);
        total_nodes += nodes;
    }

    println!("\nTotal nodes: {}\n", total_nodes);

    total_nodes
}
