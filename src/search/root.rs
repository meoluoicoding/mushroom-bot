use crate::board::Board;
use crate::movegen::RectInfo;
use crate::search::Search;
use crate::types::*;
use crate::tt::{EXACT, LOWER, UPPER};

impl Search {
    pub fn root_search(
        &mut self,
        board: &Board,
        rects: &[RectInfo],
        depth: u8,
        alpha: f32,
        beta: f32,
        root_key: u64,
        extensions_left: u8,
        ply: u8,
    ) -> (f32, Move) {
        self.record_ply(ply);
        if board.is_terminal() {
            return (board.terminal_score(), PASS);
        }
        if depth == 0 {
            let qplies = self.qsearch_plies_left(board, depth);
            return (
                if self.config.use_qsearch {
                    self.quiescence(board, alpha, beta, qplies, ply, None)
                } else {
                    board.evaluate(&self.weights, self.game_data.as_ref())
                },
                PASS,
            );
        }

        let mut best_value = f32::NEG_INFINITY;
        let mut best_action;
        let mut current_alpha = alpha;

        let tt_move = if self.config.use_tt {
            self.tt.get_best_move(root_key)
        } else {
            None
        };

        if self.config.use_futility && depth <= 2 {
            let static_eval = board.lightweight_evaluate_with_weights(&self.weights);
            let razor_margin = self.razoring_margin(depth, board.live_mask.popcount(), rects.len());
            if static_eval + razor_margin <= alpha {
                let has_forcing = rects.iter().take(8).any(|r| {
                    let mv = r.to_move();
                    self.is_tactical_move(board, mv) || self.see(board, mv, rects) > 0
                });
                if !has_forcing {
                    let qplies = self.qsearch_plies_left(board, depth);
                    return (
                        if self.config.use_qsearch {
                            self.quiescence(board, alpha, beta, qplies, ply, None)
                        } else {
                            static_eval
                        },
                        PASS,
                    );
                }
            }
        }

        let ordered_actions = self.order_moves_with_pass(board, rects, tt_move, 0, false, true, None);
        if ordered_actions.is_empty() {
            return (board.terminal_score(), PASS);
        }

        best_action = ordered_actions[0];

        let mut searched_quiets = Vec::new();
        let mut move_index = 0usize;
        for action in ordered_actions {
            self.check_timeout();
            if self.timed_out {
                break;
            }

            let child = board.apply_action(action);
            let tactical = self.is_tactical_move(board, action);
            let extend = tactical && extensions_left > 0;
            let next_extensions = if extend {
                extensions_left - 1
            } else {
                extensions_left
            };
            let extension_bonus = if extend { 1 } else { 0 };
            let mut next_depth = depth.saturating_sub(1) + extension_bonus;

            // Late Move Reduction
            let is_pass_terminal = action == PASS && board.passes == 1;
            let can_reduce = self.config.use_lmr
                && move_index > 8
                && depth >= 3
                && !tactical
                && !is_pass_terminal;
            let reduction = if can_reduce {
                self.lmr_reduction_for(depth, move_index)
            } else {
                0
            };
            if reduction > 0 {
                next_depth = next_depth.saturating_sub(reduction);
            }

            let value = if move_index == 0 {
                -self.negamax(&child, next_depth, ply + 1, -beta, -current_alpha, next_extensions, Some(action), None)
            } else {
                let mut score =
                    -self.negamax(&child, next_depth, ply + 1, -current_alpha - 1.0, -current_alpha, next_extensions, Some(action), None);
                if score > current_alpha && score < beta {
                    score = -self.negamax(&child, depth.saturating_sub(1) + extension_bonus, ply + 1, -beta, -current_alpha, next_extensions, Some(action), None);
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

            let caused_cutoff = current_alpha >= beta;
            if !tactical && action != PASS && !caused_cutoff {
                searched_quiets.push(action);
            }

            if caused_cutoff {
                self.record_history_cutoff(ply, action, depth);
                break;
            }

            move_index += 1;
        }

        if !searched_quiets.is_empty() {
            self.record_history_malus(ply, &searched_quiets, depth);
        }

        if self.config.use_tt && !self.timed_out {
            let flag = if best_value <= alpha {
                UPPER
            } else if best_value >= beta {
                LOWER
            } else {
                EXACT
            };
            self.tt
                .store(root_key, depth, best_value, flag, Some(best_action));
        }

        (best_value, best_action)
    }
}
