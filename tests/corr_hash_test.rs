use roxie::{
    board::{Board, init_ray_tables},
    evaluation::init_pesto_table,
    magics::init_magics,
    perft::perft,
    search::init_lmr_table,
    zobrist::init_zobrist,
};
use std::sync::Once;

static INIT: Once = Once::new();

fn init_engine_tables() {
    INIT.call_once(|| {
        init_zobrist();
        init_pesto_table();
        init_magics();
        init_lmr_table();
        init_ray_tables();
    });
}

// Perft on promotion / capture / castling / EP heavy positions.
// make_move() contains debug assertions comparing the incrementally
// maintained pawn/minor/major/non_pawn/material hashes against a full
// recomputation on every move, so any incremental bug panics here.
// Correct node counts also prove unmake_move restores the hashes.
#[test]
fn corr_hash_incremental_updates() {
    init_engine_tables();

    // standard perft suite positions 1-6
    let positions = [
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            4,
            197_281,
        ),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            4,
            4_085_603,
        ),
        ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 6, 11_030_083),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            4,
            422_333,
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            4,
            2_103_487,
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            4,
            3_894_594,
        ),
    ];

    for (fen, depth, expected) in positions {
        let mut board = Board::load_fen(fen);
        let nodes = perft(&mut board, depth);
        assert_eq!(nodes, expected, "perft mismatch for {}", fen);
    }
}

// All five hashes must be bit-identical before and after a full
// make/unmake walk, including promotions and underpromotions.
#[test]
fn make_unmake_restores_all_hashes() {
    init_engine_tables();

    let fens = [
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ];

    for fen in fens {
        let mut board = Board::load_fen(fen);

        let before = (
            board.get_hash(),
            board.get_pawn_hash(),
            board.get_minor_hash(),
            board.get_major_hash(),
            board.get_non_pawn_hash(),
            board.get_material_hash(),
        );

        walk(&mut board, 4);

        let after = (
            board.get_hash(),
            board.get_pawn_hash(),
            board.get_minor_hash(),
            board.get_major_hash(),
            board.get_non_pawn_hash(),
            board.get_material_hash(),
        );
        assert_eq!(
            before, after,
            "hashes changed after make/unmake walk on {}",
            fen
        );
    }
}

fn walk(board: &mut Board, depth: u32) {
    if depth == 0 {
        return;
    }

    let mut moves = board.gen_moves();
    board.filter_illegal(&mut moves);

    for i in 0..moves.len() {
        let mv = moves.moves[i];
        let undo = board.make_move(&mv);
        walk(board, depth - 1);
        board.unmake_move(&mv, &undo);
    }
}
