use crate::board::Board;
use crate::search::Search;
use crate::types::*;
use crate::tt::{EXACT, LOWER, UPPER};
use crate::movegen::fixed_rect_id;

impl Search {
    pub fn negamax(
        &mut self,
        board: &Board,
        depth: u8,
        ply: u8,
        mut alpha: f32,
        beta: f32,
        extensions_left: u8,
        prev_move: Option<Move>,
        exclude_move: Option<Move>,
    ) -> f32 {
        self.nodes += 1;
        self.check_timeout();
        self.record_ply(ply);

        if board.is_terminal() {
            return board.terminal_score();
        }
        if self.timed_out {
            return board.evaluate(&self.weights, self.game_data.as_ref());
        }
        if depth == 0 {
            let qplies = self.qsearch_plies_left(board, depth);
            return if self.config.use_qsearch {
                self.quiescence(board, alpha, beta, qplies, ply, prev_move)
            } else {
                board.evaluate(&self.weights, self.game_data.as_ref())
            };
        }

        let key = board.hash;
        let original_alpha = alpha;

        // TT probe
        if self.config.use_tt {
            if let Some((val, flag, _)) = self.tt.probe(key, depth, alpha, beta) {
                match flag {
                    EXACT => return val,
                    LOWER => alpha = alpha.max(val),
                    UPPER => {
                        if val <= alpha {
                            return val;
                        }
                    }
                    _ => {}
                }
                if alpha >= beta {
                    return alpha;
                }
            }
        }

        // Null Move Pruning (NMP)
        if self.config.use_nmp
            && depth >= 3
            && beta < f32::INFINITY
            && !self.is_zugzwang(board)
            && exclude_move.is_none()
        {
            let child = board.apply_action(PASS);
            let null_depth = depth.saturating_sub(3);
            let score = -self.negamax(&child, null_depth, ply + 1, -beta, -beta + 1.0, extensions_left, Some(PASS), None);
            if score >= beta {
                return score;
            }
        }

        let rects = self.cached_rectangles(Self::rect_cache_key(&board.values), &board.values);

        if self.config.use_futility && depth <= 2 {
            let static_eval = board.lightweight_evaluate_with_weights(&self.weights);
            let razor_margin = self.razoring_margin(depth, board.live_mask.popcount(), rects.len());
            if static_eval + razor_margin <= alpha {
                let has_forcing = rects.iter().take(8).any(|r| {
                    let mv = r.to_move();
                    self.is_tactical_move(board, mv) || self.see(board, mv, &rects) > 0
                });
                if !has_forcing {
                    let qplies = self.qsearch_plies_left(board, depth);
                    return if self.config.use_qsearch {
                        self.quiescence(board, alpha, beta, qplies, ply, prev_move)
                    } else {
                        static_eval
                    };
                }
            }
        }

        let tt_move = if self.config.use_tt { self.tt.get_best_move(key) } else { None };

        let mut singular_extended = false;
        if self.config.use_singular_extension
            && depth >= 5
            && exclude_move.is_none()
            && tt_move.is_some()
        {
            if let Some(tt_entry) = self.tt.get_entry(key) {
                if tt_entry.depth >= depth - 3
                    && (tt_entry.flag == EXACT || tt_entry.flag == LOWER)
                {
                    let mv = tt_entry.best_move.unwrap();
                    let tt_value = tt_entry.value;
                    let margin = 40.0;
                    let search_depth = depth - 3;

                    let val = self.negamax(board, search_depth, ply, tt_value - margin, tt_value - margin + 1.0, extensions_left, prev_move, Some(mv));
                    if val < tt_value - margin {
                        singular_extended = true;
                    }
                }
            }
        }

        let mut searched_tt = false;
        let mut exclude_tt_move = None;
        let mut best_value = f32::NEG_INFINITY;
        let mut best_action: Option<Move> = None;

        if let Some(mv) = tt_move {
            if Some(mv) != exclude_move && board.is_legal_action(mv) {
                self.check_timeout();
                if !self.timed_out {
                    let child = board.apply_action(mv);
                    let tactical = self.is_tactical_move(board, mv);
                    let extend = tactical && extensions_left > 0;
                    let extra = if extend { 1 } else { 0 };
                    let next_extensions = if extend {
                        extensions_left - 1
                    } else {
                        extensions_left
                    };
                    let mut next_depth = depth.saturating_sub(1) + extra;
                    if singular_extended {
                        next_depth = next_depth.saturating_add(1);
                    }

                    let score = -self.negamax(&child, next_depth, ply + 1, -beta, -alpha, next_extensions, Some(mv), None);

                    if score > best_value {
                        best_value = score;
                        best_action = Some(mv);
                    }
                    if score > alpha {
                        alpha = score;
                    }
                    if alpha >= beta {
                        self.record_history_cutoff(ply, mv, depth);
                        if let Some(pmv) = prev_move {
                            if pmv != PASS && mv != PASS {
                                let prev_id = fixed_rect_id(pmv.0, pmv.1, pmv.2, pmv.3) as usize;
                                let action_id = fixed_rect_id(mv.0, mv.1, mv.2, mv.3);
                                self.counter_moves[prev_id] = action_id;
                            }
                        }
                        if ply < 64 && mv != PASS {
                            if self.killer_moves[ply as usize][0] != Some(mv) {
                                self.killer_moves[ply as usize][1] = self.killer_moves[ply as usize][0];
                                self.killer_moves[ply as usize][0] = Some(mv);
                            }
                        }

                        if self.config.use_tt && !self.timed_out && exclude_move.is_none() {
                            self.tt.store(key, depth, best_value, LOWER, best_action);
                        }
                        return best_value;
                    }

                    searched_tt = true;
                    exclude_tt_move = Some(mv);
                }
            }
        }

        let ordered = self.order_moves_with_pass(board, &rects, tt_move, ply, false, false, prev_move);
                if ordered.is_empty() {
                    if searched_tt {
                        if self.config.use_tt && !self.timed_out && exclude_move.is_none() {
                    let flag = if best_value <= original_alpha {
                        UPPER
                    } else {
                        EXACT
                    };
                    self.tt.store(key, depth, best_value, flag, best_action);
                }
                return best_value;
            }
                    let child = board.apply_action(PASS);
                    if child.is_terminal() {
                        return board.terminal_score();
                    }
                    return -self.negamax(&child, depth.saturating_sub(1), ply + 1, -beta, -alpha, extensions_left, Some(PASS), None);
        }

        let static_eval = if self.config.use_futility && depth <= 2 {
            Some(board.evaluate(&self.weights, self.game_data.as_ref()))
        } else {
            None
        };

                let mut searched_quiets = Vec::new();
                if searched_tt {
                    if let Some(mv) = tt_move {
                        if !self.is_tactical_move(board, mv) && mv != PASS {
                            searched_quiets.push(mv);
                        }
                    }
                }

        let mut move_index = if searched_tt { 1 } else { 0 };

        for action in ordered {
            if Some(action) == exclude_move || Some(action) == exclude_tt_move {
                continue;
            }

            self.check_timeout();
            if self.timed_out {
                break;
            }

                    let tactical = self.is_tactical_move(board, action);

                    if self.config.use_futility && depth <= 2 {
                        if let Some(se) = static_eval {
                    let is_pass_terminal = action == PASS && board.passes == 1;
                    if !tactical
                        && !is_pass_terminal
                        && action != PASS
                        && Some(action) != tt_move
                    {
                        let margin = match depth {
                            1 => 120.0,
                            2 => 250.0,
                            _ => 0.0,
                        };
                                if se + margin <= alpha {
                                    move_index += 1;
                                    continue;
                                }
                            }
                        }
            }

            let child = board.apply_action(action);
            let extend = tactical && extensions_left > 0;
            let extra = if extend { 1 } else { 0 };
            let next_extensions = if extend {
                extensions_left - 1
            } else {
                extensions_left
            };
            let mut next_depth = depth.saturating_sub(1) + extra;

            let is_pass_terminal = action == PASS && board.passes == 1;
            let can_reduce = self.config.use_lmr
                && move_index > 8
                && depth >= 3
                    && !tactical
                    && !is_pass_terminal;
            let reduction = if can_reduce { self.lmr_reduction_for(depth, move_index) } else { 0 };
            if reduction > 0 {
                next_depth = next_depth.saturating_sub(reduction);
            }

            let score = if move_index == 0 {
                -self.negamax(&child, next_depth, ply + 1, -beta, -alpha, next_extensions, Some(action), None)
            } else {
                let mut s = -self.negamax(&child, next_depth, ply + 1, -alpha - 1.0, -alpha, next_extensions, Some(action), None);
                if s > alpha && s < beta {
                    s = -self.negamax(
                        &child,
                        depth.saturating_sub(1) + extra,
                        ply + 1,
                        -beta,
                        -alpha,
                        next_extensions,
                        Some(action),
                        None,
                    );
                }
                s
            };

            if score > best_value {
                best_value = score;
                best_action = Some(action);
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                self.record_history_cutoff(ply, action, depth);
                if let Some(pmv) = prev_move {
                    if pmv != PASS && action != PASS {
                        let prev_id = fixed_rect_id(pmv.0, pmv.1, pmv.2, pmv.3) as usize;
                        let action_id = fixed_rect_id(action.0, action.1, action.2, action.3);
                        self.counter_moves[prev_id] = action_id;
                    }
                }
                if ply < 64 && action != PASS {
                    if self.killer_moves[ply as usize][0] != Some(action) {
                        self.killer_moves[ply as usize][1] = self.killer_moves[ply as usize][0];
                        self.killer_moves[ply as usize][0] = Some(action);
                    }
                }
                break;
            }

            if !tactical && action != PASS {
                searched_quiets.push(action);
            }

            move_index += 1;
        }

        if !searched_quiets.is_empty() {
            self.record_history_malus(ply, &searched_quiets, depth);
        }

        if self.config.use_tt && !self.timed_out && exclude_move.is_none() {
            let flag = if best_value <= original_alpha {
                UPPER
            } else if best_value >= beta {
                LOWER
            } else {
                EXACT
            };
            self.tt.store(key, depth, best_value, flag, best_action);
        }

        best_value
    }

    pub fn quiescence(&mut self, board: &Board, mut alpha: f32, beta: f32, plies_left: u8, ply: u8, prev_move: Option<Move>) -> f32 {
        self.nodes += 1;
        self.check_timeout();
        self.record_ply(ply);

        if board.is_terminal() {
            return board.terminal_score();
        }

        let key = board.hash;
        if self.config.use_tt {
            if let Some((val, flag, _)) = self.tt.probe(key, 0, alpha, beta) {
                match flag {
                    EXACT => return val,
                    LOWER => alpha = alpha.max(val),
                    UPPER => {
                        if val <= alpha {
                            return val;
                        }
                    }
                    _ => {}
                }
                if alpha >= beta {
                    return alpha;
                }
            }
        }

        if self.timed_out || plies_left == 0 {
            let stand_pat = board.lightweight_evaluate_with_weights(&self.weights);
            if self.config.use_tt && !self.timed_out {
                self.tt.store(key, 0, stand_pat, LOWER, None);
            }
            return stand_pat;
        }

        let stand_pat = board.lightweight_evaluate_with_weights(&self.weights);
        if stand_pat >= beta {
            if self.config.use_tt && !self.timed_out {
                self.tt.store(key, 0, stand_pat, LOWER, None);
            }
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let rects = self.cached_rectangles(Self::rect_cache_key(&board.values), &board.values);
        let tt_move = if self.config.use_tt {
            self.tt.get_best_move(board.hash)
        } else {
            None
        };

        let ordered = self.order_moves_with_pass(board, &rects, tt_move, ply, false, false, prev_move);
        let mut searched = 0usize;
        let branch_limit = self.qsearch_branch_limit(board);

        for action in ordered {
            if searched >= branch_limit {
                break;
            }
            if !self.qsearch_should_search(board, action, &rects) {
                continue;
            }

            self.check_timeout();
            if self.timed_out {
                break;
            }

            let child = board.apply_action(action);
            let value = -self.quiescence(&child, -beta, -alpha, plies_left - 1, ply + 1, Some(action));
            searched += 1;

            if value >= beta {
                if self.config.use_tt && !self.timed_out {
                    self.tt.store(key, 0, beta, LOWER, Some(action));
                }
                return beta;
            }
            if value > alpha {
                alpha = value;
            }
        }

        if self.config.use_tt && !self.timed_out {
            self.tt.store(key, 0, alpha, LOWER, None);
        }

        alpha
    }
}
