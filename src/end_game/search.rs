use crate::board::Board;
use crate::search::Search;
use crate::types::*;
use crate::tt::{EXACT, LOWER, UPPER};

impl Search {
    pub fn exact_endgame_search_mtd(
        &mut self,
        board: &Board,
        first_guess: f32,
        root_key: u64,
        ply: u8,
    ) -> (f32, Move) {
        let mut g = first_guess;
        let mut upper = f32::INFINITY;
        let mut lower = f32::NEG_INFINITY;

        let rects = self.cached_rectangles(Self::rect_cache_key(&board.values), &board.values);
        let tt_move = if self.config.use_tt {
            self.tt.get_best_move(root_key)
        } else {
            None
        };

        let mut ordered_actions =
            self.order_moves_with_pass(board, &rects, tt_move, 0, false, true, None);
        if !ordered_actions.contains(&PASS) {
            ordered_actions.push(PASS);
        }
        if ordered_actions.is_empty() {
            return (board.terminal_score(), PASS);
        }

        let mut best_move = tt_move
            .and_then(|mv| ordered_actions.iter().copied().find(|&cand| cand == mv))
            .unwrap_or(ordered_actions[0]);
        let mut ordered_actions = ordered_actions;

        while lower < upper {
            let beta_val = if g == lower { g + 1.0 } else { g };
            let mut current_best_val = f32::NEG_INFINITY;
            let mut current_best_move = best_move;

            if let Some(pos) = ordered_actions.iter().position(|&mv| mv == best_move) {
                if pos != 0 {
                    ordered_actions.swap(0, pos);
                }
            }

            for &action in &ordered_actions {
                self.check_timeout();
                if self.timed_out {
                    break;
                }

                let child = board.apply_action(action);
                let val = -self.exact_negamax(&child, -beta_val, -beta_val + 1.0, ply + 1);

                if val > current_best_val {
                    current_best_val = val;
                    current_best_move = action;
                }
                if current_best_val >= beta_val {
                    break;
                }
            }

            if self.timed_out {
                break;
            }

            g = current_best_val;
            if g < beta_val {
                upper = g;
            } else {
                lower = g;
                best_move = current_best_move;
                if let Some(pos) = ordered_actions.iter().position(|&mv| mv == best_move) {
                    if pos != 0 {
                        ordered_actions.swap(0, pos);
                    }
                }
            }
        }

        if self.config.use_tt && !self.timed_out {
            self.tt.store(root_key, u8::MAX, g, EXACT, Some(best_move));
        }

        (g, best_move)
    }

    pub fn exact_endgame_search(
        &mut self,
        board: &Board,
        alpha: f32,
        beta: f32,
        root_key: u64,
        ply: u8,
    ) -> (f32, Move) {
        self.record_ply(ply);
        let rects = self.cached_rectangles(Self::rect_cache_key(&board.values), &board.values);
        if board.is_terminal() {
            return (board.terminal_score(), PASS);
        }

        let tt_move = if self.config.use_tt {
            self.tt.get_best_move(root_key)
        } else {
            None
        };

        let mut ordered_actions =
            self.order_moves_with_pass(board, &rects, tt_move, 0, false, true, None);
        if !ordered_actions.contains(&PASS) {
            ordered_actions.push(PASS);
        }
        if ordered_actions.is_empty() {
            let child = board.apply_action(PASS);
            if child.is_terminal() {
                return (board.terminal_score(), PASS);
            }
            return (-self.exact_negamax(&child, -beta, -alpha, ply + 1), PASS);
        }

        let mut best_value = f32::NEG_INFINITY;
        let mut best_action = ordered_actions[0];
        let mut current_alpha = alpha;
        let mut move_index = 0usize;

        for action in ordered_actions {
            self.check_timeout();
            if self.timed_out {
                break;
            }

            let child = board.apply_action(action);
            let value = if move_index == 0 {
                -self.exact_negamax(&child, -beta, -current_alpha, ply + 1)
            } else {
                let mut score =
                    -self.exact_negamax(&child, -current_alpha - 1.0, -current_alpha, ply + 1);
                if score > current_alpha && score < beta {
                    score = -self.exact_negamax(&child, -beta, -current_alpha, ply + 1);
                }
                score
            };

            if value > best_value {
                best_value = value;
                best_action = action;
            }
            if value > current_alpha {
                current_alpha = value;
            }
            if current_alpha >= beta {
                self.record_history_cutoff(ply, action, 1);
                break;
            }

            move_index += 1;
        }

        if self.config.use_tt && !self.timed_out && best_value.is_finite() {
            self.tt
                .store(root_key, u8::MAX, best_value, EXACT, Some(best_action));
        }

        (best_value, best_action)
    }

    pub fn exact_negamax(
        &mut self,
        board: &Board,
        mut alpha: f32,
        beta: f32,
        ply: u8,
    ) -> f32 {
        self.nodes += 1;
        self.check_timeout();
        self.record_ply(ply);

        let original_alpha = alpha;
        if board.is_terminal() {
            return board.terminal_score();
        }
        if self.timed_out {
            return board.lightweight_evaluate_with_weights(&self.weights);
        }

        let key = board.hash;
        if self.config.use_tt {
            if let Some((val, flag, _)) = self.tt.probe(key, u8::MAX, alpha, beta) {
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

        let rects = self.cached_rectangles(Self::rect_cache_key(&board.values), &board.values);
        let tt_move = if self.config.use_tt {
            self.tt.get_best_move(key)
        } else {
            None
        };

        let ordered =
            self.order_moves_with_pass(board, &rects, tt_move, ply, false, false, None);
        if ordered.is_empty() {
            let child = board.apply_action(PASS);
            if child.is_terminal() {
                return board.terminal_score();
            }
            return -self.exact_negamax(&child, -beta, -alpha, ply + 1);
        }

        let mut best_value = f32::NEG_INFINITY;
        let mut best_action: Option<Move> = None;
        let mut current_alpha = alpha;
        let mut move_index = 0usize;

        for action in ordered {
            self.check_timeout();
            if self.timed_out {
                break;
            }

            let child = board.apply_action(action);
            let score = if move_index == 0 {
                -self.exact_negamax(&child, -beta, -current_alpha, ply + 1)
            } else {
                let mut s =
                    -self.exact_negamax(&child, -current_alpha - 1.0, -current_alpha, ply + 1);
                if s > current_alpha && s < beta {
                    s = -self.exact_negamax(&child, -beta, -current_alpha, ply + 1);
                }
                s
            };

            if score > best_value {
                best_value = score;
                best_action = Some(action);
            }
            if score > current_alpha {
                current_alpha = score;
            }
            if current_alpha >= beta {
                self.record_history_cutoff(ply, action, 1);
                break;
            }

            move_index += 1;
        }

        if self.config.use_tt && !self.timed_out {
            let flag = if best_value <= original_alpha {
                UPPER
            } else if best_value >= beta {
                LOWER
            } else {
                EXACT
            };
            self.tt.store(key, u8::MAX, best_value, flag, best_action);
        }

        best_value
    }
}
