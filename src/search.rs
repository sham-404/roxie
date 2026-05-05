use crate::{
    board::Board,
    evaluation::evaluate,
    items::Move,
    transposition_table::{TTEntry, TTFlag, TranspositionTable},
};

use std::cell::Cell;

const INF: i32 = 10000000;

thread_local! {
    static NODES: Cell<u64> = Cell::new(0);
}

#[inline(always)]
fn inc_nodes() {
    NODES.with(|n| n.set(n.get() + 1));
}

pub fn search_ids(board: &mut Board, depth: u16, tt: &mut TranspositionTable) -> SearchInfo {
    NODES.with(|n| n.set(0)); // reset

    let mut info = SearchInfo {
        best_move: Move::NULL,
        best_score: 0,
        nodes: 0,
    };

    for d in 1..=depth {
        info = find_best_move(board, d, tt);
    }

    info
}

pub fn find_best_move(
    board: &mut Board,
    depth: u16,
    mut tt: &mut TranspositionTable,
) -> SearchInfo {
    let mut best_move = Move::NULL;
    let mut best_score = -INF;

    let mut move_list = board.gen_moves();

    let mut tt_move = Move::NULL;
    if let Some(entry) = tt.probe(board.get_zob_key()) {
        tt_move = entry.best_move;
    }

    for mv in move_list.with_ordering(tt_move) {
        let undo = board.make_move(&mv);
        let info = negamax(board, depth - 1, -INF, INF, 1, &mut tt);
        let cur_score = -info.best_score;
        board.unmake_move(&mv, &undo);

        if cur_score > best_score {
            best_score = cur_score;
            best_move = mv;
        }
    }

    let nodes = NODES.with(|n| n.get());

    if best_move == Move::NULL && move_list.len() > 0 {
        best_move = move_list.get(0);
    }

    SearchInfo {
        best_move,
        best_score,
        nodes,
    }
}

pub fn negamax(
    board: &mut Board,
    depth: u16,
    mut alpha: i32,
    mut beta: i32,
    ply: i32,
    tt: &mut TranspositionTable,
) -> SearchInfo {
    inc_nodes();

    // Checking draws
    if board.is_threefold() || board.is_50_rule() {
        return SearchInfo {
            best_move: Move::NULL,
            best_score: 0,
            nodes: NODES.with(|n| n.get()),
        };
    }

    // Probing the TT
    let key = board.get_zob_key();
    let mut tt_move = Move::NULL;

    if let Some(entry) = tt.probe(key) {
        tt_move = entry.best_move;
        let mut score = entry.score;

        // De-adjust mate score
        if entry.score > INF - 1000 {
            score -= ply;
        }

        if entry.score < -INF + 1000 {
            score += ply;
        }

        if entry.depth >= depth as i32 {
            match entry.flag {
                TTFlag::Exact => {
                    return SearchInfo {
                        best_move: entry.best_move,
                        best_score: score,
                        nodes: NODES.with(|n| n.get()),
                    };
                }
                TTFlag::LowerBound => alpha = alpha.max(score),
                TTFlag::UpperBound => beta = beta.min(score),
            }

            if alpha >= beta {
                return SearchInfo {
                    best_move: entry.best_move,
                    best_score: score,
                    nodes: NODES.with(|n| n.get()),
                };
            }
        }
    }

    // base case handling
    if depth == 0 {
        return SearchInfo {
            best_move: Move::NULL,
            best_score: evaluate(board),
            nodes: NODES.with(|n| n.get()),
        };
    }

    let mut move_list = board.gen_moves();
    let original_alpha = alpha;

    // checking mates
    if move_list.len() == 0 {
        let score = if board.in_check() { -INF + ply } else { 0 };
        return SearchInfo {
            best_move: Move::NULL,
            best_score: score,
            nodes: NODES.with(|n| n.get()),
        };
    }

    // NULL move pruning
    if depth >= 3 && !board.in_check() {
        let old_epsq = board.make_null_move();
        let info = negamax(board, depth - 1 - 2, -beta, -beta + 1, ply + 1, tt);
        board.unmake_null_move(old_epsq);

        let score = -info.best_score;

        if score >= beta {
            // Not returning mate scores from NMP as it can lead to false mates
            let return_score = if score >= INF - 1000 { beta } else { score };

            return SearchInfo {
                best_move: Move::NULL,
                best_score: return_score,
                nodes: NODES.with(|n| n.get()),
            };
        }
    }

    let mut max_eval = -INF;
    let mut best_move_this_node = Move::NULL;

    for mv in move_list.with_ordering(tt_move) {
        let undo = board.make_move(&mv);
        let info = negamax(board, depth - 1, -beta, -alpha, ply + 1, tt);

        let eval = -info.best_score;
        board.unmake_move(&mv, &undo);

        if eval > max_eval {
            max_eval = eval;
            best_move_this_node = mv;
        }

        if eval > alpha {
            alpha = eval;
        }

        // pruning
        if alpha >= beta {
            break;
        }
    }

    let flag = if max_eval <= original_alpha {
        TTFlag::UpperBound
    } else if max_eval >= beta {
        TTFlag::LowerBound
    } else {
        TTFlag::Exact
    };

    // Adjusting for mate score
    let mut score_to_store = max_eval;
    if score_to_store > INF - 1000 {
        score_to_store += ply;
    }
    if score_to_store < -INF + 1000 {
        score_to_store -= ply;
    }

    tt.store(TTEntry {
        key,
        depth: depth as i32,
        score: score_to_store,
        flag,
        best_move: best_move_this_node,
    });

    return SearchInfo {
        best_move: best_move_this_node,
        best_score: max_eval,
        nodes: NODES.with(|n| n.get()),
    };
}

pub struct SearchInfo {
    pub best_move: Move,
    pub best_score: i32,
    pub nodes: u64,
}
