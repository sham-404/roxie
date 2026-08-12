use crate::{
    board::{Board, mask},
    r#const::{BLACK_PAWN_ATTACKS, KING_ATTACKS, KNIGHT_ATTACKS, MAX_PLY, WHITE_PAWN_ATTACKS},
    engine::Engine,
    items::{Color, Move, MoveFlag, MoveList, Piece, PieceInfo},
    magics::{get_bishop_move_bits, get_rook_move_bits},
    move_pick::MovePicker,
    tt::{TTEntry, TTFlag},
    uci::{GoControl, MAX_DEPTH},
    uci_print,
};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use std::sync::LazyLock;

const PIECE_VALS_FOR_SEE: [i32; 6] = [100, 320, 330, 500, 900, 10_000];
pub const MAX_MOVES: usize = 256;

pub static LMR_TABLE: LazyLock<[[i32; MAX_MOVES]; MAX_PLY]> = LazyLock::new(|| {
    let mut table = [[0; MAX_MOVES]; MAX_PLY];

    for depth in 1..MAX_PLY {
        for mv_idx in 1..MAX_MOVES {
            let reduction = 1.5 + (depth as f64).ln() * (mv_idx as f64).ln() / 1.5;
            table[depth][mv_idx] = reduction as i32;
        }
    }

    table
});

pub fn init_lmr_table() {
    LazyLock::force(&LMR_TABLE);
}

pub const MATE: i16 = 27_000;
pub const INF: i16 = 27_500;
pub const MAX_HISTORY: i32 = 20000;

impl Engine {
    pub fn search_ids<F>(&mut self, limits: &SearchLimits, mut on_iteration: F) -> SearchInfo
    where
        F: FnMut(&SearchInfo),
    {
        let mut info = SearchInfo::new();

        self.killers = [[Move::NULL; 2]; MAX_PLY];
        self.setup_accumulator();
        self.tt.inc_generation();

        let mut last_complete_info = info.clone();

        // Iterative Deepening Search loop
        for d in 1..=limits.depth.unwrap_or(MAX_DEPTH) {
            let mut best_move: Move;
            let mut best_score: i16;
            let last_iteration_nodes = info.nodes;
            info.nodes += 1;

            // making the search of depth 1 completely mandatory
            // as it guarentees us to return a valid move
            info.is_mandatory = 1 == d;

            // Aspiration window setup
            let mut delta = 50i32; // Use i32 for safe math
            let mut alpha = -INF;
            let mut beta = INF;

            // using aspiration window only on relatively higher depths
            if d > 5 {
                alpha = (last_complete_info.score as i32 - delta).max(-INF as i32) as i16;
                beta = (last_complete_info.score as i32 + delta).min(INF as i32) as i16;
            }

            // aspiration re-search loop
            loop {
                let orig_alpha = alpha;
                let orig_beta = beta;

                let move_list = self.board.gen_moves();

                best_move = if move_list.len() != 0 {
                    if let Some(&mv) = move_list
                        .as_slice()
                        .iter()
                        .find(|&&mv| self.board.is_legal_mv(mv))
                    {
                        mv
                    } else {
                        Move::NULL
                    }
                } else {
                    Move::NULL
                };
                best_score = -INF;

                let mut tt_move = Move::NULL;
                info.stats.tt_probes += 1;
                if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
                    info.stats.tt_hits += 1;
                    tt_move = entry.best_move();
                }

                let mut picker =
                    MovePicker::new(tt_move, [Move::NULL, Move::NULL], Move::NULL, false);
                let mut mv_searched = 0;

                while let Some(mv) = self.pick_next_mv(&mut picker) {
                    let undo = self.board.make_move(&mv);
                    if self.board.in_check_after_moving() {
                        self.board.unmake_move(&mv, &undo);
                        continue;
                    }

                    self.update_nnue(&mv, &undo, 0);

                    let mv_idx = mv_searched;
                    mv_searched += 1;

                    // PV search //
                    // full window search on first move (tt_move)
                    let score = if mv_idx == 0 {
                        -self.negamax(
                            SearchParams {
                                depth: d - 1,
                                alpha: -beta,
                                beta: -alpha,
                                ply: 1,
                                extension: 0,
                                prev_move: mv,
                                excluded_move: Move::NULL,
                            },
                            &limits,
                            &mut info,
                        )
                    } else {
                        // Null window search
                        let mut eval = -self.negamax(
                            SearchParams {
                                depth: d - 1,
                                alpha: -alpha - 1,
                                beta: -alpha,
                                ply: 1,
                                extension: 0,
                                prev_move: mv,
                                excluded_move: Move::NULL,
                            },
                            &limits,
                            &mut info,
                        );

                        // Research if the move looks promising
                        if eval > alpha {
                            eval = -self.negamax(
                                SearchParams {
                                    depth: d - 1,
                                    alpha: -beta,
                                    beta: -alpha,
                                    ply: 1,
                                    extension: 0,
                                    prev_move: mv,
                                    excluded_move: Move::NULL,
                                },
                                &limits,
                                &mut info,
                            );
                        }

                        eval
                    };
                    // PV search //

                    self.board.unmake_move(&mv, &undo);

                    if info.abort {
                        break;
                    }

                    if score > best_score {
                        best_score = score;
                        best_move = mv;
                    }

                    if score > alpha {
                        alpha = score;
                    }
                }

                if info.abort {
                    break;
                }

                // aspiration failed low
                if best_score <= orig_alpha {
                    self.tt.store(TTEntry {
                        key: self.board.get_zob_key(),
                        depth: d,
                        score: best_score as i32,
                        flag: TTFlag::UpperBound,
                        best_move,
                        age: self.tt.get_generation(),
                    });

                    alpha = (orig_alpha as i32 - delta).max(-INF as i32) as i16;
                    beta = orig_beta;
                    delta = (delta * 2).min(INF as i32); // clamp delta
                    continue;
                }

                // aspiration failed high
                if best_score >= orig_beta {
                    alpha = orig_alpha;
                    beta = (orig_beta as i32 + delta).min(INF as i32) as i16;
                    delta = (delta * 2).min(INF as i32); // clamp delta
                    continue;
                }

                // successful aspiration search
                break;
            }

            // if aborted, dont update the whole result
            if info.abort {
                last_complete_info.nodes = info.nodes;
                last_complete_info.seldepth = info.seldepth;
                last_complete_info.stats = info.stats.clone();
                last_complete_info.depth = d;
                last_complete_info.pv = self.gen_pv();

                // last info print if search is aborted midway
                on_iteration(&last_complete_info);
                break;
            }

            // // safety check
            // if best_move == Move::NULL {
            //     let mv_list = self.board.gen_moves();
            //     if mv_list.len() != 0 {
            //         best_move = mv_list.get(0);
            //     }
            // }

            // Manual storing for root node in TT
            let root_key = self.board.get_zob_key();
            self.tt.store(TTEntry {
                key: root_key,
                depth: d,
                score: best_score as i32,
                flag: TTFlag::Exact,
                best_move,
                age: self.tt.get_generation(),
            });

            info.depth = d;
            info.score = best_score;
            info.best_move = best_move;
            info.pv = self.gen_pv();

            info.stats.nodes_by_depth[d as usize] =
                info.nodes as usize - last_iteration_nodes as usize;
            info.stats.nodes = info.nodes as usize;

            last_complete_info = info.clone();

            on_iteration(&info);

            // if it reached the solf limit, searching further is probably useless
            if let Some(time) = limits.soft_time {
                if time <= info.start_time.elapsed() {
                    break;
                }
            }
        }

        last_complete_info
    }

    fn negamax(
        &mut self,
        params: SearchParams,
        limits: &SearchLimits,
        info: &mut SearchInfo,
    ) -> i16 {
        info.check_limits(limits);

        let SearchParams {
            depth,
            mut alpha,
            mut beta,
            ply,
            extension,
            prev_move,
            excluded_move,
        } = params;

        if info.abort {
            return alpha;
        }

        info.nodes += 1;
        info.seldepth = info.seldepth.max(ply as u16);

        // Mate Distance Pruning //
        alpha = alpha.max(-MATE + ply as i16);
        beta = beta.min(MATE - ply as i16);

        // Prune if mate score is found and it cannot be improved
        if alpha >= beta {
            return alpha;
        }
        // Mate Distance Pruning //

        // checking if maximum ply is reached
        if ply + 1 >= MAX_PLY as i32 {
            return self.evaluate(ply as usize);
        }

        // Checking draws
        if self.board.is_draw() {
            return 0;
        }

        // Probing the TT
        let key = self.board.get_zob_key();
        let mut tt_move = Move::NULL;
        let mut tt_depth = 0;
        let mut tt_score = 0;
        let mut tt_flag = TTFlag::Exact;

        info.stats.tt_probes += 1;

        if let Some(entry) = self.tt.probe(key) {
            info.stats.tt_hits += 1;

            tt_move = entry.best_move();
            tt_depth = entry.depth();
            tt_flag = entry.flag();
            tt_score = entry.score();

            // De-adjust mate score
            if tt_score > MATE - MAX_PLY as i16 {
                tt_score -= ply as i16;
            }
            if tt_score < -MATE + MAX_PLY as i16 {
                tt_score += ply as i16;
            }

            // ONLY return early if we are not in a singular search
            if entry.depth() >= depth && excluded_move == Move::NULL {
                match entry.flag() {
                    TTFlag::Exact => {
                        info.stats.tt_exact_cutoffs += 1;
                        return tt_score;
                    }
                    TTFlag::LowerBound => alpha = alpha.max(tt_score),
                    TTFlag::UpperBound => beta = beta.min(tt_score),
                }

                if alpha >= beta {
                    info.stats.tt_bound_cutoffs += 1;
                    return tt_score;
                }
            }
        }

        //// Internal Iterative Deepening (IID)
        // If we don't have a tt move, try to search a lower depth search and hope it
        // probes a tt move
        if depth >= 5 && tt_move == Move::NULL && excluded_move == Move::NULL {
            info.stats.iid_attempts += 1;

            let iid_depth = depth - 2;

            self.negamax(
                SearchParams {
                    depth: iid_depth,
                    alpha,
                    beta,
                    ply,
                    extension: 0,
                    prev_move,
                    excluded_move: Move::NULL,
                },
                limits,
                info,
            );

            // probing tt, as we might have a move there now
            if let Some(entry) = self.tt.probe(key) {
                info.stats.iid_success += 1;
                tt_move = entry.best_move();
            }
        }
        //// Internal Iterative Deepening (IID)

        // base case handling
        if depth == 0 {
            return self.quiescence(
                SearchParams {
                    depth,
                    alpha,
                    beta,
                    ply,
                    extension,
                    prev_move,
                    excluded_move,
                },
                info,
                limits,
            );
        }

        let in_check = self.board.in_check();
        let static_eval = self.evaluate(ply as usize); // static evaluation

        self.eval_history.store(static_eval, in_check, ply as usize);

        //// Reverse Futility Pruning (Static Null Move Pruning) //
        if !in_check
            && depth <= 4
            && beta.abs() < MATE - MAX_PLY as i16
            && excluded_move == Move::NULL
        {
            info.stats.rfp_attempts += 1;

            let margin = depth as i16 * 120; // 120 cp per depth as margin

            if static_eval - margin >= beta {
                info.stats.rfp_cutoffs += 1;
                return static_eval; // Immediate static beta cutoff
            }
        }
        //// Reverse Futility Pruning (Static Null Move Pruning) //

        //// NULL move pruning
        if let Some(cutoff_score) = self.nmp_search(
            SearchParams {
                depth,
                alpha,
                beta,
                ply,
                extension,
                prev_move,
                excluded_move,
            },
            limits,
            info,
            static_eval,
        ) {
            return cutoff_score;
        }
        //// NULL move pruning

        //// ProbCut (Probablistic Cut)
        if depth >= 5
            && !in_check
            && excluded_move == Move::NULL
            && beta.abs() < MATE - MAX_PLY as i16
        {
            let pc_margin = 200i32;
            let pc_beta = ((beta as i32 + pc_margin).min((MATE - MAX_PLY as i16) as i32)) as i16;
            let pc_depth = if depth >= 8 { depth - 4 } else { depth - 3 };

            let tt_move = if tt_move.flag().is_quiet() {
                Move::NULL
            } else {
                tt_move
            };

            let mut pc_picker = MovePicker::new(tt_move, [Move::NULL; 2], prev_move, true);

            while let Some(mv) = self.pick_next_mv(&mut pc_picker) {
                if !mv.flag().is_promo() && self.board.see(&mv) <= 0 {
                    continue;
                }

                let undo = self.board.make_move(&mv);
                if self.board.in_check_after_moving() {
                    self.board.unmake_move(&mv, &undo);
                    continue;
                }

                info.stats.probcut_attempts += 1;
                self.update_nnue(&mv, &undo, ply as usize);

                let pc_score = -self.negamax(
                    SearchParams {
                        depth: pc_depth,
                        alpha: -pc_beta,
                        beta: -pc_beta + 1, // Zero window
                        ply: ply + 1,
                        extension: 0,
                        prev_move: mv,
                        excluded_move: Move::NULL,
                    },
                    limits,
                    info,
                );

                self.board.unmake_move(&mv, &undo);

                if pc_score >= pc_beta {
                    info.stats.probcut_cutoffs += 1;

                    // Guard against fake mates from shallow searches
                    let safe_score = if pc_score >= MATE - MAX_PLY as i16 {
                        pc_beta
                    } else {
                        pc_score
                    };

                    self.tt.store(TTEntry {
                        key: self.board.get_zob_key(),
                        depth: pc_depth + 1, // Safe depth assumption
                        score: safe_score as i32,
                        flag: TTFlag::LowerBound,
                        best_move: mv,
                        age: self.tt.get_generation(),
                    });

                    return safe_score;
                }
            }
        }
        //// ProbCut (Probablistic Cut)

        //// Singular Extension
        let mut se_extension = 0;

        // Only trigger on high depths, when we aren't already doing a singular search,
        // when we have a valid TT move, and when the TT depth is sufficient.
        if depth >= 8
            && tt_move != Move::NULL
            && params.excluded_move == Move::NULL
            && tt_depth >= depth - 3
            && tt_flag != TTFlag::UpperBound // We need a reliable lower bound for the TT move
            && tt_score.abs() < MATE - MAX_PLY as i16
        // Don't extend mates
        {
            let singular_margin = depth as i16 * 2;
            let singular_beta = (tt_score - singular_margin).max(-MATE);

            // Zero-window search at reduced depth, excluding the TT move
            let se_score = self.negamax(
                SearchParams {
                    depth: depth / 2, // Standard singular reduction
                    alpha: singular_beta - 1,
                    beta: singular_beta,
                    ply,
                    extension: 0,
                    prev_move,
                    excluded_move: tt_move, // exclude the TT move
                },
                limits,
                info,
            );

            // If the search failed low, it means no other move can even reach
            // the TT score minus the margin. Thus TT move is singular
            if se_score < singular_beta {
                se_extension = 1;
            }
        }
        //// Singular Extension

        let original_alpha = alpha;

        let mut max_eval = -INF;
        let mut best_move_this_node = Move::NULL;
        let mut fail_high = false;
        let mut quiet_list = MoveList::new();
        let mut quiet_searched = 0;

        //// Actual searching loop
        let mut picker = MovePicker::new(tt_move, self.killers[ply as usize], prev_move, false);
        let mut mv_searched = 0;

        while let Some(mv) = self.pick_next_mv(&mut picker) {
            if mv == excluded_move {
                continue;
            }

            let mv_idx = mv_searched;

            let flag = mv.flag();
            let is_quiet = flag.is_quiet();

            if is_quiet && mv_idx > 0 {
                // late move pruning //
                let is_non_pv = alpha + 1 == beta;
                let mut lmp_threshold = 3 + (depth * depth) as usize;
                let is_improving =
                    self.eval_history
                        .is_improving(static_eval, in_check, ply as usize);

                if !is_improving {
                    lmp_threshold /= 2;
                }

                if is_non_pv && depth <= 5 && !in_check {
                    info.stats.lmp_attempts += 1;

                    if quiet_searched >= lmp_threshold {
                        info.stats.lmp_prunes += 1;
                        picker.skip_quiets();
                        continue;
                    }
                }
                // late move pruning //
            } else if mv_idx > 0 {
                // SEE pruning //
                let is_non_pv = alpha + 1 == beta;

                if depth <= 5
                    && is_non_pv
                    && !in_check
                    && mv != tt_move
                    && flag.is_capture()
                    && !flag.is_promo()
                    && !self.board.gives_check(mv)
                {
                    info.stats.see_prune_attempts += 1;
                    let margin = depth as i32 * 80;

                    if self.board.see(&mv) < -margin {
                        info.stats.see_prunes_happened += 1;
                        continue;
                    }
                }
                // SEE pruning //
            }

            // Futility Pruning //
            if depth < 3 && mv_idx > 0 && is_quiet && !in_check {
                // If static eval + margin can't even beat alpha,
                // this quiet move is highly unlikely to change the node status.

                let futility_margin = depth as i16 * 150;
                if static_eval + futility_margin <= alpha {
                    // We must verify the move doesn't give a check before skipping it
                    // for safeplay

                    info.stats.futility_attempts += 1;

                    if !self.board.gives_check(mv) {
                        info.stats.futility_prunes += 1;
                        continue;
                    }
                }
            }
            // Futility Pruning //

            let undo = self.board.make_move(&mv);
            if self.board.in_check_after_moving() {
                self.board.unmake_move(&mv, &undo);
                continue;
            }

            if is_quiet {
                quiet_list.push(mv);
                quiet_searched += 1
            }

            mv_searched += 1;

            self.update_nnue(&mv, &undo, ply as usize);

            // Taking account for extended depth for singular extention
            let next_depth = if mv == tt_move {
                depth + se_extension
            } else {
                depth
            };

            let eval = self.pv_search(
                mv,
                mv_idx,
                quiet_searched,
                SearchParams {
                    depth: next_depth,
                    alpha,
                    beta,
                    ply,
                    extension,
                    prev_move,
                    excluded_move: Move::NULL,
                },
                limits,
                info,
            );

            self.board.unmake_move(&mv, &undo);

            if eval > max_eval {
                max_eval = eval;
                best_move_this_node = mv;
            }

            if eval > alpha {
                alpha = eval;
            }

            // pruning (beta cutoff)
            if eval >= beta {
                info.stats.beta_cutoffs += 1;
                info.stats.cutoffs_by_idx[mv_idx] += 1;

                fail_high = true;

                if is_quiet {
                    self.store_killer(mv, ply as usize);

                    let stm = self.board.side_to_move().val();
                    let bonus = (depth * depth).min(400) as i32;

                    // history maluses
                    for q_mv in quiet_list.as_slice() {
                        if *q_mv == mv {
                            continue;
                        }

                        // for history
                        let h = &mut self.history[stm][q_mv.from()][q_mv.to()];
                        *h -= bonus >> 2;
                        *h = (*h).clamp(-MAX_HISTORY, MAX_HISTORY);

                        // for continuation history
                        self.continuation_history.update(
                            &self.board,
                            prev_move,
                            *q_mv,
                            -bonus >> 2,
                        );
                    }

                    // history bonus scoring
                    let h = &mut self.history[stm][mv.from()][mv.to()];
                    *h += bonus;
                    *h = (*h).clamp(-MAX_HISTORY, MAX_HISTORY);

                    // continuation history scoring
                    self.continuation_history
                        .update(&self.board, prev_move, mv, bonus);

                    // counter moves storing
                    self.counter_moves.store(prev_move, mv);
                }

                break;
            }
        }

        // checking mates
        if mv_searched == 0 {
            return if in_check { -MATE + ply as i16 } else { 0 };
        }

        let flag = if fail_high {
            TTFlag::LowerBound
        } else if max_eval <= original_alpha {
            TTFlag::UpperBound
        } else {
            TTFlag::Exact
        };

        // Adjusting for mate score
        let mut score_to_store = max_eval;
        if score_to_store > MATE - MAX_PLY as i16 {
            score_to_store += ply as i16;
        }
        if score_to_store < -MATE + MAX_PLY as i16 {
            score_to_store -= ply as i16;
        }

        // dont store in the tt if excluded_move move has some move
        if !info.abort && excluded_move == Move::NULL {
            self.tt.store(TTEntry {
                key,
                depth: depth,
                score: score_to_store as i32,
                flag,
                best_move: best_move_this_node,
                age: self.tt.get_generation(),
            });
        }

        max_eval
    }

    fn nmp_search(
        &mut self,
        params: SearchParams,
        limits: &SearchLimits,
        info: &mut SearchInfo,
        static_eval: i16,
    ) -> Option<i16> {
        let SearchParams {
            depth,
            beta,
            ply,
            extension,
            excluded_move,
            ..
        } = params;

        // Conditions for NMP
        if depth > 3
            && excluded_move == Move::NULL
            && beta.abs() < MATE - MAX_PLY as i16
            && static_eval >= beta
            && !self.board.in_check()
            && !self.board.is_endgame()
        {
            info.stats.nmp_attemps += 1;

            let r = 3 + depth / 6;
            let nmp_depth = (depth - 1).saturating_sub(r);

            let old_epsq = self.board.make_null_move();
            self.update_nnue_null_move(ply as usize);

            // Zero-window search
            let score = -self.negamax(
                SearchParams {
                    depth: nmp_depth,
                    alpha: -beta,
                    beta: -beta + 1,
                    ply: ply + 1,
                    extension: extension,
                    prev_move: Move::NULL,
                    excluded_move: Move::NULL,
                },
                limits,
                info,
            );

            self.board.unmake_null_move(old_epsq);

            if score >= beta {
                info.stats.nmp_cutoffs += 1;
                // Not returning mate scores from NMP as it can lead to false mates
                return Some(if score >= MATE - MAX_PLY as i16 {
                    beta
                } else {
                    score
                });
            }
        }

        None
    }

    fn pv_search(
        &mut self,
        mv: Move,
        mv_idx: usize,
        quiet_searched: usize,
        params: SearchParams,
        limits: &SearchLimits,
        info: &mut SearchInfo,
    ) -> i16 {
        info.check_limits(limits);

        let SearchParams {
            depth,
            alpha,
            beta,
            ply,
            extension,
            prev_move,
            ..
        } = params;

        let in_check = self.board.in_check();

        let (extension, next_extensions) = if in_check && extension < 2 {
            (1, extension + 1)
        } else {
            (0, extension)
        };

        // searching first move with full window
        if mv_idx == 0 {
            return -self.negamax(
                SearchParams {
                    depth: depth - 1 + extension,
                    alpha: -beta,
                    beta: -alpha,
                    ply: ply + 1,
                    extension: next_extensions,
                    prev_move: mv,
                    excluded_move: Move::NULL,
                },
                limits,
                info,
            );
        }

        // checking whether lmr is applicable
        let can_reduce = quiet_searched > 1 && depth > 3 && !in_check && mv.flag().is_quiet();

        // Late move reduction //
        let reduction = if can_reduce {
            info.stats.lmr_attempts += 1;

            // Safely fetch base reduction from table
            let table_depth = (depth as usize).min(MAX_PLY - 1);
            let table_idx = mv_idx.min(MAX_MOVES - 1);
            let mut r = LMR_TABLE[table_depth][table_idx];

            // Continuous history adjustment
            // max adjustment of +/- 4 plies
            let adjustment_fac = MAX_HISTORY / 4;
            let stm = self.board.side_to_move().opponent().val();
            let cont_hist = self
                .continuation_history
                .get_after_mv(&self.board, prev_move, mv) as i32;
            let hist_score = self.history[stm][mv.from()][mv.to()] + cont_hist;

            let hist_adjustment = hist_score / adjustment_fac;

            r -= hist_adjustment.clamp(-4, 4);

            // Counter and killer move adjustment
            if mv == self.counter_moves.get(prev_move) || self.killers[ply as usize].contains(&mv) {
                r -= 1;
            }

            // Minimum reduction of 1 ply, maximum of depth - 1 to avoid negative depths
            r.clamp(1, depth as i32 - 1) as u16
        } else {
            0
        };
        // Late move reduction //

        // Null window search
        let mut eval = -self.negamax(
            SearchParams {
                depth: depth - 1 - reduction + extension,
                alpha: -alpha - 1,
                beta: -alpha,
                ply: ply + 1,
                extension: next_extensions,
                prev_move: mv,
                excluded_move: Move::NULL,
            },
            limits,
            info,
        );

        // re-search if fail high
        if reduction > 0 && eval > alpha {
            info.stats.lmr_research += 1;

            eval = -self.negamax(
                SearchParams {
                    depth: depth - 1 + extension,
                    alpha: -alpha - 1,
                    beta: -alpha,
                    ply: ply + 1,
                    extension: next_extensions,
                    prev_move: mv,
                    excluded_move: Move::NULL,
                },
                limits,
                info,
            );
        }

        // full window re-search if needed
        if eval > alpha && eval < beta {
            eval = -self.negamax(
                SearchParams {
                    depth: depth - 1 + extension,
                    alpha: -beta,
                    beta: -alpha,
                    ply: ply + 1,
                    extension: next_extensions,
                    prev_move: mv,
                    excluded_move: Move::NULL,
                },
                limits,
                info,
            );
        }

        eval
    }

    fn quiescence(
        &mut self,
        params: SearchParams,
        info: &mut SearchInfo,
        limits: &SearchLimits,
    ) -> i16 {
        info.check_limits(limits);

        let SearchParams {
            mut alpha,
            mut beta,
            ply,
            prev_move,
            ..
        } = params;

        if info.abort {
            return alpha;
        }

        info.nodes += 1;
        info.stats.q_nodes += 1;
        info.seldepth = info.seldepth.max(ply as u16);

        // Mate Distance Pruning //
        alpha = alpha.max(-MATE + ply as i16);
        beta = beta.min(MATE - ply as i16);

        // Prune if mate score is found and it cannot be improved
        if alpha >= beta {
            return alpha;
        }

        // Mate Distance Pruning //
        // max ply checking
        if ply + 1 >= MAX_PLY as i32 {
            return self.evaluate(ply as usize);
        }

        // Checking draws
        if self.board.is_draw() {
            return 0;
        }

        let mut tt_move = Move::NULL;
        let mut tt_score = None;
        let mut tt_flag = None;

        info.stats.tt_probes += 1;
        if let Some(entry) = self.tt.probe(self.board.get_zob_key()) {
            info.stats.tt_hits += 1;
            tt_move = entry.best_move();

            let mut score = entry.score();
            // De-adjust mate score
            if entry.score() > MATE - MAX_PLY as i16 {
                score -= ply as i16;
            }
            if entry.score() < -MATE + MAX_PLY as i16 {
                score += ply as i16;
            }

            tt_score = Some(score);
            tt_flag = Some(entry.flag());

            match entry.flag() {
                TTFlag::Exact => {
                    info.stats.tt_exact_cutoffs += 1;
                    return score;
                }
                TTFlag::LowerBound => alpha = alpha.max(score),
                TTFlag::UpperBound => beta = beta.min(score),
            }

            if alpha >= beta {
                info.stats.tt_bound_cutoffs += 1;
                return score;
            }
        }

        let in_check = self.board.in_check();
        let mut best_score = -INF;

        // Stand pat
        let mut stand_pat = self.evaluate(ply as usize);
        if !in_check {
            // Cap the stand_pat score using TT bounds
            if let Some(score) = tt_score {
                if tt_flag == Some(TTFlag::UpperBound) && stand_pat > score {
                    stand_pat = score;
                }
                if tt_flag == Some(TTFlag::LowerBound) && stand_pat < score {
                    stand_pat = score;
                }
            }

            best_score = stand_pat;

            // beta cutoff
            if stand_pat >= beta {
                self.q_tt_store(TTFlag::LowerBound, stand_pat, tt_move, ply);
                return stand_pat;
            }

            if stand_pat > alpha {
                alpha = stand_pat;
            }
        }

        let qsearch_picker = !in_check;
        let orig_alpha = alpha;
        let mut best_move_this_node = Move::NULL;

        let mut picker = MovePicker::new(
            tt_move,
            self.killers[ply as usize],
            prev_move,
            qsearch_picker,
        );
        let mut mv_searched = 0;

        while let Some(mv) = self.pick_next_mv(&mut picker) {
            let gives_check = self.board.gives_check(mv);
            if !in_check && !mv.flag().is_promo() {
                // soft delta pruning (see pruning) //
                if !gives_check && self.board.see(&mv) < 0 {
                    continue;
                }
                // soft delta pruning (see pruning) //

                // delta pruning //
                let gain = if mv.flag() == MoveFlag::EN_PASSANT {
                    100
                } else {
                    let cap_piece = self.board.piece_on(mv.to());
                    self.board.get_see_value(cap_piece)
                } as i16;

                const DELTA: i16 = 200;
                if !gives_check && stand_pat + gain + DELTA < alpha {
                    continue;
                }
                // delta pruning //
            }

            let undo = self.board.make_move(&mv);
            if self.board.in_check_after_moving() {
                self.board.unmake_move(&mv, &undo);
                continue;
            }
            let mv_idx = mv_searched;
            mv_searched += 1;

            self.update_nnue(&mv, &undo, ply as usize);

            let score = -self.quiescence(
                SearchParams {
                    alpha: -beta,
                    beta: -alpha,
                    ply: ply + 1,
                    prev_move: mv,
                    ..params
                },
                info,
                limits,
            );

            self.board.unmake_move(&mv, &undo);

            if info.abort {
                return best_score;
            }

            if score > best_score {
                best_move_this_node = mv;
                best_score = score;
            }

            // beta cutoff
            if score >= beta {
                info.stats.q_beta_cutoffs += 1;
                info.stats.q_cutoffs_by_idx[mv_idx] += 1;

                self.q_tt_store(TTFlag::LowerBound, score, mv, ply);
                return score;
            }

            if score > alpha {
                alpha = score;
            }
        }

        if !qsearch_picker && mv_searched == 0 {
            let score = -MATE + ply as i16;
            self.q_tt_store(TTFlag::Exact, score, tt_move, ply);
            return score;
        }

        let flag = if best_score <= orig_alpha {
            TTFlag::UpperBound
        } else {
            TTFlag::Exact
        };

        if !info.abort {
            self.q_tt_store(flag, best_score, best_move_this_node, ply);
        }

        best_score
    }
}

impl Engine {
    #[inline(always)]
    pub fn with_ordering(
        &mut self,
        tt_move: Move,
        prev_move: Move,
        ply: usize,
        movelist: &mut MoveList,
    ) {
        let counter_mv = self.counter_moves.get(prev_move);
        let killer_1 = self.killers[ply][0];
        let killer_2 = self.killers[ply][1];

        for i in 0..movelist.len() {
            let mv = movelist.moves[i];

            if mv == tt_move {
                // Give it a score higher than any possible capture/promotion
                movelist.score[i] = 1_000_000_000;
            } else if mv.flag().is_quiet() {
                if mv == killer_1 {
                    movelist.score[i] = 40_000_000;
                    continue;
                } else if mv == killer_2 {
                    movelist.score[i] = 39_000_000;
                    continue;
                } else if mv == counter_mv {
                    movelist.score[i] = 38_000_000;
                    continue;
                }

                let mut score = self.board.score_move(mv);
                score += self.history[self.board.side_to_move().val()][mv.from()][mv.to()] << 6;
                score += (self.continuation_history.get(&self.board, prev_move, mv) << 6) as i32;

                movelist.score[i] = score;
            } else {
                movelist.score[i] = self.board.score_move(mv);
            }
        }
    }

    fn q_tt_store(&mut self, flag: TTFlag, score: i16, mv: Move, ply: i32) {
        // Adjusting for mate score
        let mut score_to_store = score;
        if score_to_store > MATE - MAX_PLY as i16 {
            score_to_store += ply as i16;
        }
        if score_to_store < -MATE + MAX_PLY as i16 {
            score_to_store -= ply as i16;
        }

        self.tt.store(TTEntry {
            key: self.board.get_zob_key(),
            depth: 0,
            score: score_to_store as i32,
            flag,
            best_move: mv,
            age: self.tt.get_generation(),
        });
    }

    #[inline(always)]
    fn store_killer(&mut self, mv: Move, ply: usize) {
        if self.killers[ply][0] != mv {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = mv;
        }
    }

    fn gen_pv(&mut self) -> Vec<Move> {
        let mut pv = Vec::new();
        let mut undo_stack = Vec::new();
        let mut visited_keys = Vec::new();

        loop {
            let key = self.board.get_zob_key();
            if let Some(entry) = self.tt.probe(key) {
                let mv = entry.best_move();

                // Stoping if the move is empty or we hit an infinite transposition cycle
                if mv == Move::NULL || visited_keys.contains(&key) {
                    break;
                }

                // Verify the TT move is actually valid in this position
                // (needed due to rare hash collisions overwritings)
                let mut moves = self.board.gen_moves();
                self.board.filter_illegal(&mut moves);
                if !moves.as_slice().contains(&mv) {
                    break;
                }

                pv.push(mv);
                visited_keys.push(key);

                // Make the move on the board to advance the state and get the next Zobrist key
                let undo = self.board.make_move(&mv);
                undo_stack.push((mv, undo));
            } else {
                break;
            }
        }

        // Unmake all moves in reverse order to restore the board state to the root
        while let Some((mv, undo)) = undo_stack.pop() {
            self.board.unmake_move(&mv, &undo);
        }

        pv
    }
}

#[derive(Clone, Copy)]
struct SearchParams {
    depth: u16,
    alpha: i16,
    beta: i16,
    ply: i32,
    extension: u8,
    prev_move: Move,
    excluded_move: Move,
}

#[derive(Clone, Copy)]
pub struct SearchStats {
    pub nodes: usize,
    pub q_nodes: usize,

    // +1 is necessary, as depth is usually calculated from 1, so we
    // have to include the MAX_DEPTH idx as well
    pub nodes_by_depth: [usize; MAX_DEPTH as usize + 1],

    pub nmp_attemps: usize,
    pub nmp_cutoffs: usize,

    pub lmr_attempts: usize,
    pub lmr_research: usize,

    pub lmp_attempts: usize,
    pub lmp_prunes: usize,

    pub see_prune_attempts: usize,
    pub see_prunes_happened: usize,

    pub rfp_attempts: usize,
    pub rfp_cutoffs: usize,

    pub probcut_attempts: usize,
    pub probcut_cutoffs: usize,

    pub iid_attempts: usize,
    pub iid_success: usize,

    pub futility_attempts: usize,
    pub futility_prunes: usize,

    pub tt_probes: usize,
    pub tt_hits: usize,
    pub tt_exact_cutoffs: usize,
    pub tt_bound_cutoffs: usize,

    pub beta_cutoffs: usize,
    pub q_beta_cutoffs: usize,

    pub cutoffs_by_idx: [usize; MAX_MOVES],
    pub q_cutoffs_by_idx: [usize; MAX_MOVES],
}

impl SearchStats {
    pub fn new() -> SearchStats {
        Self {
            nodes: 0,
            q_nodes: 0,
            nodes_by_depth: [0; MAX_DEPTH as usize + 1],

            nmp_attemps: 0,
            nmp_cutoffs: 0,
            lmr_attempts: 0,
            lmr_research: 0,
            lmp_attempts: 0,
            lmp_prunes: 0,
            see_prune_attempts: 0,
            see_prunes_happened: 0,
            rfp_attempts: 0,
            rfp_cutoffs: 0,
            probcut_attempts: 0,
            probcut_cutoffs: 0,
            iid_attempts: 0,
            iid_success: 0,
            futility_attempts: 0,
            futility_prunes: 0,
            tt_probes: 0,
            tt_hits: 0,
            tt_exact_cutoffs: 0,
            tt_bound_cutoffs: 0,
            beta_cutoffs: 0,
            q_beta_cutoffs: 0,
            cutoffs_by_idx: [0; MAX_MOVES],
            q_cutoffs_by_idx: [0; MAX_MOVES],
        }
    }

    #[inline]
    fn pct(num: usize, denom: usize) -> f64 {
        if denom == 0 {
            0.0
        } else {
            num as f64 * 100.0 / denom as f64
        }
    }

    #[inline]
    fn ratio(num: usize, denom: usize) -> f64 {
        if denom == 0 {
            0.0
        } else {
            num as f64 / denom as f64
        }
    }

    pub fn print_stats(&self) {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                     ROXIE SEARCH STATS                       ║");
        println!("╚══════════════════════════════════════════════════════════════╝");

        // ------------------------------------------------------------
        // BASIC SEARCH
        // ------------------------------------------------------------

        println!();
        println!("── Search ─────────────────────────────────────────────────────");

        let total_nodes = self.nodes;
        let q_nodes = self.q_nodes;
        let main_nodes = total_nodes.saturating_sub(q_nodes);

        println!("Main search nodes:      {}", main_nodes);
        println!("QSearch nodes:          {}", q_nodes);
        println!("Total nodes:            {}", total_nodes);

        println!(
            "QSearch / main:         {:.2}%",
            Self::pct(q_nodes, main_nodes)
        );

        println!(
            "QSearch / total:        {:.2}%",
            Self::pct(q_nodes, total_nodes)
        );

        println!(
            "Main / total:           {:.2}%",
            Self::pct(main_nodes, total_nodes)
        );

        // ------------------------------------------------------------
        // BETA CUToffs
        // ------------------------------------------------------------

        println!();
        println!("── Cutoffs ────────────────────────────────────────────────────");

        println!("Main beta cutoffs:      {}", self.beta_cutoffs);

        println!("QSearch beta cutoffs:   {}", self.q_beta_cutoffs);

        println!(
            "All searched cutoffs:   {}",
            self.beta_cutoffs + self.q_beta_cutoffs
        );

        // TT cutoffs are deliberately kept separate.
        let tt_cutoffs = self.tt_exact_cutoffs + self.tt_bound_cutoffs;

        println!("TT exact cutoffs:       {}", self.tt_exact_cutoffs);

        println!("TT bound cutoffs:       {}", self.tt_bound_cutoffs);

        println!("TT total cutoffs:       {}", tt_cutoffs);

        println!(
            "TT cutoffs / probes:    {:.3}%",
            Self::pct(tt_cutoffs, self.tt_probes)
        );

        // ------------------------------------------------------------
        // MOVE ORDERING
        // ------------------------------------------------------------

        println!();
        println!("── Move Ordering ──────────────────────────────────────────────");

        println!(
            "{:<8} {:>12} {:>12} {:>12} {:>12}",
            "Index", "Main", "Main Cum.", "QSearch", "Q Cum."
        );

        let mut main_cumulative = 0.0;
        let mut q_cumulative = 0.0;

        let mut main_weighted_index = 0usize;
        let mut q_weighted_index = 0usize;

        for idx in 0..MAX_MOVES {
            let cutoff = self.cutoffs_by_idx[idx];
            let q_cutoff = self.q_cutoffs_by_idx[idx];

            main_weighted_index += idx * cutoff;
            q_weighted_index += idx * q_cutoff;

            let distribution = Self::pct(cutoff, self.beta_cutoffs);
            let q_distribution = Self::pct(q_cutoff, self.q_beta_cutoffs);

            main_cumulative += distribution;
            q_cumulative += q_distribution;

            if distribution == 0.0 && q_distribution == 0.0 {
                break;
            }

            println!(
                "{:<8} {:>11.3}% {:>11.3}% {:>11.3}% {:>11.3}%",
                idx, distribution, main_cumulative, q_distribution, q_cumulative
            );
        }

        println!();

        println!(
            "Average main cutoff index: {:.3}",
            Self::ratio(main_weighted_index, self.beta_cutoffs)
        );

        println!(
            "Average qsearch cutoff index: {:.3}",
            Self::ratio(q_weighted_index, self.q_beta_cutoffs)
        );

        println!(
            "Main cutoffs in move 0:    {:.3}%",
            Self::pct(self.cutoffs_by_idx[0], self.beta_cutoffs)
        );

        println!(
            "Main cutoffs in first 2:   {:.3}%",
            Self::pct(self.cutoffs_by_idx.iter().take(2).sum(), self.beta_cutoffs)
        );

        println!(
            "Main cutoffs in first 4:   {:.3}%",
            Self::pct(self.cutoffs_by_idx.iter().take(4).sum(), self.beta_cutoffs)
        );

        println!(
            "Main cutoffs in first 8:   {:.3}%",
            Self::pct(self.cutoffs_by_idx.iter().take(8).sum(), self.beta_cutoffs)
        );

        println!(
            "Q cutoffs in move 0:       {:.3}%",
            Self::pct(self.q_cutoffs_by_idx[0], self.q_beta_cutoffs)
        );

        println!(
            "Q cutoffs in first 2:      {:.3}%",
            Self::pct(
                self.q_cutoffs_by_idx.iter().take(2).sum(),
                self.q_beta_cutoffs
            )
        );

        println!(
            "Q cutoffs in first 4:      {:.3}%",
            Self::pct(
                self.q_cutoffs_by_idx.iter().take(4).sum(),
                self.q_beta_cutoffs
            )
        );

        // ------------------------------------------------------------
        // NULL MOVE PRUNING
        // ------------------------------------------------------------

        println!();
        println!("── Null Move Pruning ──────────────────────────────────────────");

        println!("Attempts:                 {}", self.nmp_attemps);

        println!("Cutoffs:                  {}", self.nmp_cutoffs);

        println!(
            "Cutoff efficiency:        {:.3}%",
            Self::pct(self.nmp_cutoffs, self.nmp_attemps)
        );

        // ------------------------------------------------------------
        // LMR
        // ------------------------------------------------------------

        println!();
        println!("── Late Move Reductions ───────────────────────────────────────");

        println!("Attempts:                 {}", self.lmr_attempts);

        println!("Re-searches:              {}", self.lmr_research);

        println!(
            "Re-search rate:           {:.3}%",
            Self::pct(self.lmr_research, self.lmr_attempts)
        );

        println!(
            "Accepted reductions:      {:.3}%",
            Self::pct(
                self.lmr_attempts.saturating_sub(self.lmr_research),
                self.lmr_attempts
            )
        );

        // ------------------------------------------------------------
        // LMP
        // ------------------------------------------------------------

        println!();
        println!("── Late Move Pruning ──────────────────────────────────────────");

        println!("Attempts:                 {}", self.lmp_attempts);

        println!("Pruned:                   {}", self.lmp_prunes);

        println!(
            "Prune efficiency:         {:.3}%",
            Self::pct(self.lmp_prunes, self.lmp_attempts)
        );

        println!(
            "Searched instead:         {:.3}%",
            Self::pct(
                self.lmp_attempts.saturating_sub(self.lmp_prunes),
                self.lmp_attempts
            )
        );

        // ------------------------------------------------------------
        // SEE
        // ------------------------------------------------------------

        println!();
        println!("── SEE Pruning ────────────────────────────────────────────────");

        println!("Attempts:                 {}", self.see_prune_attempts);

        println!("Pruned:                   {}", self.see_prunes_happened);

        println!(
            "Prune efficiency:         {:.3}%",
            Self::pct(self.see_prunes_happened, self.see_prune_attempts)
        );

        // ------------------------------------------------------------
        // RFP
        // ------------------------------------------------------------

        println!();
        println!("── Reverse Futility Pruning ──────────────────────────────────");

        println!("Attempts:                 {}", self.rfp_attempts);

        println!("Cutoffs:                  {}", self.rfp_cutoffs);

        println!(
            "Cutoff efficiency:        {:.3}%",
            Self::pct(self.rfp_cutoffs, self.rfp_attempts)
        );

        // ------------------------------------------------------------
        // PROBCUT
        // ------------------------------------------------------------

        println!();
        println!("── ProbCut ────────────────────────────────────────────────────");

        println!("Attempts:                 {}", self.probcut_attempts);

        println!("Cutoffs:                  {}", self.probcut_cutoffs);

        println!(
            "Cutoff efficiency:        {:.3}%",
            Self::pct(self.probcut_cutoffs, self.probcut_attempts)
        );

        // ------------------------------------------------------------
        // IID
        // ------------------------------------------------------------

        println!();
        println!("── Internal Iterative Deepening ───────────────────────────────");

        println!("Attempts:                 {}", self.iid_attempts);

        println!("Successes:                {}", self.iid_success);

        println!(
            "Success rate:             {:.3}%",
            Self::pct(self.iid_success, self.iid_attempts)
        );

        // ------------------------------------------------------------
        // FUTILITY
        // ------------------------------------------------------------

        println!();
        println!("── Futility Pruning ───────────────────────────────────────────");

        println!("Attempts:                 {}", self.futility_attempts);

        println!("Pruned:                   {}", self.futility_prunes);

        println!(
            "Prune efficiency:         {:.3}%",
            Self::pct(self.futility_prunes, self.futility_attempts)
        );

        println!(
            "Searched instead:         {:.3}%",
            Self::pct(
                self.futility_attempts.saturating_sub(self.futility_prunes),
                self.futility_attempts
            )
        );

        // ------------------------------------------------------------
        // TRANSPOSITION TABLE
        // ------------------------------------------------------------

        println!();
        println!("── Transposition Table ────────────────────────────────────────");

        println!("Probes:                   {}", self.tt_probes);

        println!("Hits:                     {}", self.tt_hits);

        println!(
            "Hit rate:                 {:.3}%",
            Self::pct(self.tt_hits, self.tt_probes)
        );

        println!("Exact cutoffs:            {}", self.tt_exact_cutoffs);

        println!("Bound cutoffs:            {}", self.tt_bound_cutoffs);

        println!("Total cutoffs:            {}", tt_cutoffs);

        println!(
            "Exact / hits:             {:.3}%",
            Self::pct(self.tt_exact_cutoffs, self.tt_hits)
        );

        println!(
            "Bound / hits:             {:.3}%",
            Self::pct(self.tt_bound_cutoffs, self.tt_hits)
        );

        // ------------------------------------------------------------
        // EBF
        // ------------------------------------------------------------

        println!();
        println!("── Effective Branching Factor ─────────────────────────────────");

        let mut ebf_sum = 0.0;
        let mut ebf_count = 0;

        for depth in 1..MAX_PLY + 1 {
            let prev = self.nodes_by_depth[depth - 1];
            let current = self.nodes_by_depth[depth];

            if prev == 0 || current == 0 {
                continue;
            }

            let ebf = current as f64 / prev as f64;

            println!("Depth {:>2} -> {:>2}:       {:.3}", depth - 1, depth, ebf);

            ebf_sum += ebf;
            ebf_count += 1;
        }

        if ebf_count > 0 {
            println!(
                "Average EBF:              {:.3}",
                ebf_sum / ebf_count as f64
            );
        } else {
            println!("EBF unavailable: node/depth data not recorded");
        }

        // ------------------------------------------------------------
        // SUMMARY
        // ------------------------------------------------------------

        println!();
        println!("── Search Summary ─────────────────────────────────────────────");

        println!(
            "Pruning attempts:         {}",
            self.nmp_attemps
                + self.lmp_attempts
                + self.see_prune_attempts
                + self.rfp_attempts
                + self.probcut_attempts
                + self.futility_attempts
        );

        println!(
            "Successful prunes:        {}",
            self.nmp_cutoffs
                + self.lmp_prunes
                + self.see_prunes_happened
                + self.rfp_cutoffs
                + self.probcut_cutoffs
                + self.futility_prunes
        );

        println!("Total TT cutoffs:         {}", tt_cutoffs);

        println!("Total beta cutoffs:       {}", self.beta_cutoffs);

        println!("Total qsearch cutoffs:    {}", self.q_beta_cutoffs);

        println!();
        println!("════════════════════════════════════════════════════════════════");
    }

    pub fn describe(&self) {
        println!("q nodes: {}", self.q_nodes);

        println!("NMP attempted: {}", self.nmp_attemps);
        println!("NMP cutoffs: {}", self.nmp_cutoffs);

        println!("LMR attempted: {}", self.lmr_attempts);
        println!("LMR researched: {}", self.lmr_research);

        println!("LMP attempted: {}", self.lmp_attempts);
        println!("LMP pruned: {}", self.lmp_prunes);

        println!("IID attempted: {}", self.iid_attempts);
        println!("IID succeeded: {}", self.iid_success);

        println!("SEE attempted: {}", self.see_prune_attempts);
        println!("SEE pruned: {}", self.see_prunes_happened);

        println!("RFP attempted: {}", self.rfp_attempts);
        println!("RFP cutoffs: {}", self.rfp_cutoffs);

        println!("Prob Cut attempted: {}", self.probcut_attempts);
        println!("Prob Cut cutoffs: {}", self.probcut_cutoffs);

        println!("Futility attempted: {}", self.futility_attempts);
        println!("Futility pruned: {}", self.futility_prunes);

        println!("TT Probes: {}", self.tt_probes);
        println!("TT Hits: {}", self.tt_hits);
        println!("TT exact cutoffs: {}", self.tt_exact_cutoffs);
        println!("TT bound cutoffs: {}", self.tt_bound_cutoffs);
        println!();
    }
}

#[derive(Clone)]
pub struct SearchInfo {
    pub start_time: Instant,
    pub depth: u16,
    pub seldepth: u16,
    pub score: i16,
    pub best_move: Move,
    pub nodes: u64,
    pub abort: bool,
    pub is_mandatory: bool,
    pub pv: Vec<Move>,

    pub stats: SearchStats,
}

impl SearchInfo {
    pub fn new() -> SearchInfo {
        SearchInfo {
            start_time: Instant::now(),
            best_move: Move::NULL,
            depth: 0,
            seldepth: 0,
            score: 0,
            nodes: 0,
            abort: false,
            is_mandatory: true,
            pv: Vec::new(),

            stats: SearchStats::new(),
        }
    }

    pub fn get_mate_depth(&self) -> Option<i16> {
        if self.score > MATE - MAX_PLY as i16 {
            return Some((MATE - self.score + 1) / 2);
        } else if self.score < -MATE + MAX_PLY as i16 {
            return Some(-((MATE + self.score + 1) / 2));
        } else {
            return None;
        };
    }

    pub fn print(&self) {
        let mut pv_str = String::new();
        for mv in &self.pv {
            pv_str.push_str(&format!("{} ", mv.to_coord()));
        }

        let score = if let Some(mate) = self.get_mate_depth() {
            format!("mate {}", mate)
        } else {
            format!("cp {}", self.score)
        };

        uci_print!(
            "info depth {} seldepth {} score {} nodes {} nps {} time {} pv {}",
            self.depth,
            self.seldepth,
            score,
            self.nodes,
            self.nps(),
            self.start_time.elapsed().as_millis(),
            pv_str,
        );
    }

    fn nps(&self) -> u64 {
        let ms = self.start_time.elapsed().as_millis().max(1);
        self.nodes * 1000 / ms as u64
    }

    fn check_limits(&mut self, limits: &SearchLimits) {
        // not checking time limits if it is a mandatory search
        if self.is_mandatory {
            return;
        }

        // cheking if stop command is made
        if limits.stop_signal.load(Ordering::Relaxed) {
            self.abort = true;
            return;
        }

        // checking if mate depth is enabled and reached
        if let Some(mate) = limits.mate {
            match self.get_mate_depth() {
                Some(m) if m > 0 && m <= mate as i16 => {
                    self.abort = true;
                }
                _ => {}
            }
        }

        // checking once in a while if the time limit is reached
        if self.nodes & 2047 == 0 {
            if let Some(nodes) = limits.nodes {
                if self.nodes >= nodes {
                    self.abort = true;
                }
            }

            if let Some(limit) = limits.hard_time {
                if self.start_time.elapsed() >= limit {
                    self.abort = true;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SearchLimits {
    pub depth: Option<u16>,
    pub nodes: Option<u64>,
    pub mate: Option<u16>,
    pub hard_time: Option<Duration>,
    pub soft_time: Option<Duration>,
    pub infinite: bool,
    pub start_time: Instant,

    pub stop_signal: Arc<AtomicBool>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            soft_time: None,
            hard_time: None,
            depth: None,
            nodes: None,
            mate: None,
            infinite: false,
            start_time: Instant::now(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl SearchLimits {
    pub fn from_go(ctrl: &GoControl, stm: Color) -> Self {
        let mut limits = SearchLimits::default();
        limits.depth = ctrl.depth;
        limits.infinite = ctrl.infinite;

        if ctrl.infinite {
            return limits;
        }

        // Fixed Nodes
        if let Some(nodes) = ctrl.nodes {
            limits.nodes = Some(nodes);
            return limits;
        }

        // Fixed mate depth
        if let Some(mate) = ctrl.mate {
            limits.mate = Some(mate as u16);
            return limits;
        }

        // Fixed movetime
        if let Some(ms) = ctrl.movetime {
            let duration = Duration::from_millis(ms);

            limits.soft_time = Some(duration);
            limits.hard_time = Some(duration);

            return limits;
        }

        // Normal clock management
        let (time_left, increment) = match stm {
            Color::White => (ctrl.wtime.unwrap_or(0), ctrl.winc.unwrap_or(0)),
            Color::Black => (ctrl.btime.unwrap_or(0), ctrl.binc.unwrap_or(0)),
        };

        // Subtract 50ms to account for GUI/Network communication time.
        let safe_time_left = time_left.saturating_sub(50);

        if safe_time_left > 0 {
            let moves_to_go = ctrl.movestogo.unwrap_or(30);

            // Base allocation: spread remaining safe time over expected remaining moves
            let base_time = safe_time_left / moves_to_go;

            // Use 3/4 of the increment (standard aggressive time management)
            let inc_time = increment * 3 / 4;

            let allocated = base_time + inc_time;

            // Soft Limit: Stop starting new depths early (60% of allocated)
            limits.soft_time = Some(Duration::from_millis(allocated * 6 / 10));

            // Hard Limit: min 20 max 80 % of safe_time
            let hard_limit = (allocated * 2).max(20).min(safe_time_left * 8 / 10);

            limits.hard_time = Some(Duration::from_millis(hard_limit));
        } else if time_left > 0 {
            // EMERGENCY MODE: We have less than 50ms on the actual clock
            // Give it 1ms soft time, and whatever is physically left on the clock (minus a tiny 5ms buffer).
            limits.soft_time = Some(Duration::from_millis(1));
            limits.hard_time = Some(Duration::from_millis(time_left.saturating_sub(5).max(1)));
        }

        limits
    }

    pub fn with_depth(depth: u16) -> Self {
        let mut limits = Self::default();
        limits.depth = Some(depth);
        limits
    }

    pub fn with_movetime(movetime: u64) -> Self {
        let mut limits = Self::default();
        limits.soft_time = Some(Duration::from_millis(movetime));
        limits.hard_time = Some(Duration::from_millis(movetime));
        limits
    }
}

impl Board {
    fn attacks_to(&self, to_pos: usize, all_occ: u64) -> u64 {
        let mut attackers = 0u64;

        let king = Piece::KING;
        let queen = Piece::QUEEN;
        let rook = Piece::ROOK;
        let bishop = Piece::BISHOP;
        let knight = Piece::KNIGHT;
        let pawn = Piece::PAWN;

        let white = Piece::WHITE;
        let black = Piece::BLACK;

        // bishop or queen is attacking
        let diag_atks = get_bishop_move_bits(to_pos, all_occ);
        attackers |= diag_atks
            & (self.bb(white | bishop)
                | self.bb(black | bishop)
                | self.bb(white | queen)
                | self.bb(black | queen))
            & all_occ;

        // rook or queen is attacking
        let straight_atk = get_rook_move_bits(to_pos, all_occ);
        attackers |= straight_atk
            & (self.bb(white | rook)
                | self.bb(black | rook)
                | self.bb(white | queen)
                | self.bb(black | queen))
            & all_occ;

        //knight is attacking
        attackers |=
            KNIGHT_ATTACKS[to_pos] & (self.bb(white | knight) | self.bb(black | knight)) & all_occ;

        // King attacks
        attackers |=
            KING_ATTACKS[to_pos] & (self.bb(white | king) | self.bb(black | king)) & all_occ;

        // if a opp pawn is in cur color pawn's attacking sq, then
        // the opponent pawn is attacking the current sq
        attackers |= WHITE_PAWN_ATTACKS[to_pos] & self.bb(black | pawn) & all_occ;
        attackers |= BLACK_PAWN_ATTACKS[to_pos] & self.bb(white | pawn) & all_occ;

        attackers
    }

    // Code written by refering the algorithm provided in chess programming wiki
    // link -> https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    pub fn see(&self, mov: &Move) -> i32 {
        if mov.flag() == MoveFlag::EN_PASSANT {
            return 100;
        }

        let mut gain = [0; 32];
        let mut d = 0;

        let from = mov.from();
        let to = mov.to();

        let target = self.piece_on(to);
        let mut cur_victim = self.piece_on(from); // victim coz it moved to the to_sq

        // Initial gain = captured piece
        gain[0] = self.get_see_value(target);

        if mov.flag().is_promo() {
            let promo_piece = mov.flag().get_promo_piece();
            gain[0] += self.get_see_value(promo_piece) - self.get_see_value(Piece::PAWN);
            cur_victim = promo_piece;
        }

        // Simulated occupancy AFTER first capture
        let mut occ = self.all_occ();
        occ ^= mask(from); // piece moved from from_sq

        let mut side = self.side_to_move().opponent();

        loop {
            // Find all attackers in current position
            let attackers = self.attacks_to(to, occ);
            let (from_set, piece) = self.get_least_valuable_piece(attackers, occ, side);

            if from_set == 0 {
                break;
            }

            d += 1;

            gain[d] = self.get_see_value(cur_victim) - gain[d - 1];

            // SEE pruning
            if (-gain[d - 1]).max(gain[d]) < 0 {
                // if we are pruning, then we are discarding this branch
                // so no we have to reduce it by one depth
                d -= 1;
                break;
            }

            cur_victim = piece; // next victim is the cur attacker

            // Remove this attacker from occupancy
            occ ^= from_set;
            side = side.opponent();
        }

        // Backward minimax pass
        while d > 0 {
            d -= 1;
            gain[d] = -((-gain[d]).max(gain[d + 1]));
        }

        gain[0]
    }

    fn get_least_valuable_piece(&self, attackers: u64, occ: u64, side: Color) -> (u64, PieceInfo) {
        let color_mask = if side == Color::White {
            Piece::WHITE
        } else {
            Piece::BLACK
        };

        let my_attackers = attackers & occ & self.occ(&side);

        for p_type in [
            Piece::PAWN,
            Piece::KNIGHT,
            Piece::BISHOP,
            Piece::ROOK,
            Piece::QUEEN,
            Piece::KING,
        ] {
            let subset = my_attackers & self.bb(p_type | color_mask);

            if subset != 0 {
                let lsb = subset & subset.wrapping_neg();

                return (lsb, p_type | color_mask);
            }
        }

        (0, Piece::NONE)
    }

    fn get_see_value(&self, piece: PieceInfo) -> i32 {
        if piece == Piece::NONE {
            return 0;
        }

        let idx = Piece::to_idx(piece) % 6;
        // Piece indices: 0:P, 1:N, 2:B, 3:R, 4:Q, 5:K
        PIECE_VALS_FOR_SEE[idx]
    }
}
