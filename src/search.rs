use crate::{
    board::{Board, mask},
    r#const::{BLACK_PAWN_ATTACKS, KING_ATTACKS, KNIGHT_ATTACKS, MAX_PLY, WHITE_PAWN_ATTACKS},
    engine::Engine,
    items::{Color, Move, MoveFlag, MoveList, Piece, PieceInfo},
    square::Square,
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

pub const MAX_MOVES: usize = 256;

pub static LMR_TABLE: LazyLock<[[i32; MAX_MOVES]; MAX_PLY]> = LazyLock::new(|| {
    let mut table = [[0; MAX_MOVES]; MAX_PLY];

    for depth in 1..MAX_PLY {
        for mv_idx in 1..MAX_MOVES {
            // The constants 1.0 and 1.5 tunable
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
                info.nodes += 1;
                let orig_alpha = alpha;
                let orig_beta = beta;

                let mut move_list = self.board.gen_moves();

                best_move = if move_list.len() != 0 {
                    move_list.get(0)
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

                self.with_ordering(tt_move, Move::NULL, 0, &mut move_list);
                for mv_idx in 0..move_list.len() {
                    let mv = move_list.pick_move(mv_idx);
                    let undo = self.board.make_move(&mv);
                    self.update_nnue(&mv, &undo, 0);

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

        // Checking draws
        if self.board.is_threefold() || self.board.is_50_rule() {
            return 0;
        }

        // Probing the TT
        let key = self.board.get_zob_key();
        let mut tt_move = Move::NULL;
        info.stats.tt_probes += 1;

        if let Some(entry) = self.tt.probe(key) {
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

            if entry.depth() >= depth {
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
        }

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
                },
                info,
                limits,
            );
        }

        let in_check = self.board.in_check();
        let static_eval = self.evaluate(ply as usize); // static evaluation

        // Reverse Futility Pruning (Static Null Move Pruning) //
        if !in_check && depth <= 4 && beta.abs() < MATE - MAX_PLY as i16 {
            info.stats.rfp_attempts += 1;

            let margin = depth as i16 * 120; // 120 cp per depth as margin

            if static_eval - margin >= beta {
                info.stats.rfp_cutoffs += 1;
                return static_eval; // Immediate static beta cutoff
            }
        }
        // Reverse Futility Pruning (Static Null Move Pruning) //

        // NULL move pruning
        if let Some(cutoff_score) = self.nmp_search(
            SearchParams {
                depth,
                alpha,
                beta,
                ply,
                extension,
                prev_move,
            },
            limits,
            info,
        ) {
            return cutoff_score;
        }
        // NULL move pruning

        let mut move_list = self.board.gen_moves();
        let original_alpha = alpha;

        // checking mates
        if move_list.len() == 0 {
            return if in_check { -MATE + ply as i16 } else { 0 };
        }

        let mut max_eval = -INF;
        let mut best_move_this_node = Move::NULL;
        let mut fail_high = false;
        let mut quiet_list = MoveList::new();
        let mut quiet_searched = 0;

        // Actual searching loop

        self.with_ordering(tt_move, prev_move, ply as usize, &mut move_list);
        for mv_idx in 0..move_list.len() {
            let mv = move_list.pick_move(mv_idx);
            let flag = mv.flag();
            let is_quiet = flag.is_quiet();

            if is_quiet {
                quiet_searched += 1;
                quiet_list.push(mv);

                // late move pruning //
                let is_non_pv = alpha + 1 == beta;
                let lmp_threshold = 3 + (depth * depth) as usize / 3;

                // disabled currently!
                if is_non_pv
                    && false
                    && !in_check
                    && depth <= 5
                    && quiet_searched > lmp_threshold
                    && !self.killers[ply as usize].contains(&mv)
                {
                    continue;
                }
                // late move pruning //
            } else {
                // SEE pruning //
                let is_non_pv = alpha + 1 == beta;

                if depth <= 5
                    && is_non_pv
                    && !in_check
                    && mv != tt_move
                    && flag.is_capture()
                    && !self.board.gives_check(mv)
                {
                    let margin = depth as i32 * 80;

                    if self.board.see(&mv) < -margin {
                        continue;
                    }
                }
                // SEE pruning //
            }

            // Futility Pruning //
            if depth < 3 && mv_idx > 0 && is_quiet && !in_check {
                // If static eval + margin can't even beat alpha,
                // this quiet move is highly unlikely to change the node status.
                // Margin scales up with depth: Depth 1 = 150cp, Depth 2 = 300cp, Depth 3 = 450cp

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
            self.update_nnue(&mv, &undo, ply as usize);

            let eval = self.pv_search(
                mv,
                mv_idx,
                quiet_searched,
                SearchParams {
                    depth,
                    alpha,
                    beta,
                    ply,
                    extension,
                    prev_move,
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

            // pruning
            if eval >= beta {
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
                        self.continuation_history
                            .update(&self.board, prev_move, *q_mv, -bonus >> 2);
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

        if !info.abort {
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
    ) -> Option<i16> {
        let SearchParams {
            depth,
            beta,
            ply,
            extension,
            ..
        } = params;

        // Conditions for NMP
        if depth > 3
            && !self.board.in_check()
            && !self.board.is_endgame()
            && self.evaluate(ply as usize) >= beta
        {
            info.stats.nmp_attemps += 1;

            let r = 3 + depth / 6;
            let old_epsq = self.board.make_null_move();
            self.update_nnue_null_move(ply as usize);

            // Zero-window search
            let score = -self.negamax(
                SearchParams {
                    depth: depth - 1 - r,
                    alpha: -beta,
                    beta: -beta + 1,
                    ply: ply + 1,
                    extension: extension,
                    prev_move: Move::NULL,
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

            r -= hist_adjustment;

            // Counter move adjustment
            if mv == self.counter_moves.get(prev_move) {
                r -= 1;
            }

            // killer move adjustment
            if self.killers[ply as usize].contains(&mv) {
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
                self.q_tt_store(TTFlag::LowerBound, stand_pat, Move::NULL, ply);
                return stand_pat;
            }

            if stand_pat > alpha {
                alpha = stand_pat;
            }
        }

        let mut move_list = if in_check {
            let mv_list = self.board.gen_moves();
            if mv_list.len() == 0 {
                let score = -MATE + ply as i16;
                self.q_tt_store(TTFlag::Exact, score, Move::NULL, ply);
                return score;
            }
            mv_list
        } else {
            self.board.gen_cap_moves()
        };

        let orig_alpha = alpha;
        let mut best_move_this_node = Move::NULL;

        self.with_ordering(tt_move, prev_move, ply as usize, &mut move_list);
        for mv_idx in 0..move_list.len() {
            let mv = move_list.pick_move(mv_idx);
            if !in_check && !mv.flag().is_promo() {
                // soft delta pruning (see pruning) //
                if self.board.see(&mv) < 0 && !self.board.gives_check(mv) {
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
                if stand_pat + gain + DELTA < alpha {
                    continue;
                }
                // delta pruning //
            }

            let undo = self.board.make_move(&mv);

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

            if score >= beta {
                self.q_tt_store(TTFlag::LowerBound, score, mv, ply);
                return score;
            }

            if score > alpha {
                alpha = score;
            }
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
    #[inline]
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

    #[inline]
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
                let moves = self.board.gen_moves();
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
}

#[derive(Clone, Copy)]
pub struct SearchStats {
    pub q_nodes: usize,

    pub nmp_attemps: usize,
    pub nmp_cutoffs: usize,

    pub lmr_attempts: usize,
    pub lmr_research: usize,

    pub rfp_attempts: usize,
    pub rfp_cutoffs: usize,

    pub futility_attempts: usize,
    pub futility_prunes: usize,

    pub tt_probes: usize,
    pub tt_hits: usize,
    pub tt_exact_cutoffs: usize,
    pub tt_bound_cutoffs: usize,
}

impl SearchStats {
    pub fn new() -> SearchStats {
        SearchStats {
            q_nodes: 0,

            nmp_attemps: 0,
            nmp_cutoffs: 0,

            lmr_attempts: 0,
            lmr_research: 0,

            rfp_attempts: 0,
            rfp_cutoffs: 0,

            futility_attempts: 0,
            futility_prunes: 0,

            tt_probes: 0,
            tt_hits: 0,
            tt_exact_cutoffs: 0,
            tt_bound_cutoffs: 0,
        }
    }

    pub fn describe(&self) {
        println!("q nodes: {}", self.q_nodes);

        println!("NMP attempted: {}", self.nmp_attemps);
        println!("NMP cutoffs: {}", self.nmp_cutoffs);

        println!("LMR attempted: {}", self.lmr_attempts);
        println!("LMR researched: {}", self.lmr_research);

        println!("RFP attempted: {}", self.rfp_attempts);
        println!("RFP cutoffs: {}", self.rfp_cutoffs);

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

        // Sliding pieces
        let directions = [
            ([(1, 1), (1, -1), (-1, 1), (-1, -1)], true), // diagonals
            ([(1, 0), (-1, 0), (0, 1), (0, -1)], false),  // straight
        ];
        let from = Square::new(to_pos);

        for (dir, is_diag) in directions {
            for (dr, df) in dir {
                let mut sq = from;

                while let Some(next) = sq.offset(dr, df) {
                    let to_bb = mask(next.index());

                    // Some piece is blocking our way
                    if to_bb & all_occ != 0 {
                        let piece = self.piece_on(next.index());
                        let piece_type = Piece::get_type(piece);
                        if piece_type == Piece::QUEEN
                            || (piece_type == Piece::BISHOP && is_diag)
                            || (piece_type == Piece::ROOK && !is_diag)
                        {
                            attackers |= to_bb;
                        }

                        // accumulate attackers if the blocking piece is an enemy rook,
                        // bishop or a queen, else break the loop as we have
                        // been blocked by our own piece, or an non sliding
                        // enemy piece

                        break;
                    }
                    sq = next;
                }
            }
        }

        //knight is attacking
        attackers |= KNIGHT_ATTACKS[to_pos]
            & (self.bb(Piece::WHITE | Piece::KNIGHT) | self.bb(Piece::BLACK | Piece::KNIGHT))
            & all_occ;

        // King attacks
        attackers |= KING_ATTACKS[to_pos]
            & (self.bb(Piece::WHITE | Piece::KING) | self.bb(Piece::BLACK | Piece::KING))
            & all_occ;

        // if a opp pawn is in cur color pawn's attacking sq, then
        // the opponent pawn is attacking the current sq
        attackers |= WHITE_PAWN_ATTACKS[to_pos] & self.bb(Piece::BLACK | Piece::PAWN) & all_occ;
        attackers |= BLACK_PAWN_ATTACKS[to_pos] & self.bb(Piece::WHITE | Piece::PAWN) & all_occ;

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
        match idx {
            0 => 100,
            1 => 320,
            2 => 330,
            3 => 500,
            4 => 900,
            _ => 10000,
        }
    }
}
