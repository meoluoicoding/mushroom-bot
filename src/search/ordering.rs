use crate::board::Board;
use crate::dataloader::GameData;
use crate::movegen::{fixed_rect_id, generate_rectangles, RectInfo};
use crate::mquality::MoveQualityTable;
use crate::search::Search;
use crate::side::GameSide;
use crate::types::*;
use crate::eval::{compute_move_features, static_move_score};
use crate::opening::ordering::opening_move_bonus;
use crate::mid_game::ordering::midgame_move_bonus;
use crate::end_game::ordering::endgame_move_bonus;

const ROOT_REPLY_TOP_K: usize = 12;
const DEFENSIVE_LIVE_THRESHOLD: u32 = 16;
const DEFENSIVE_LEAD_THRESHOLD: i32 = 8;

pub fn should_consider_pass(board: &Board, rects: &[RectInfo]) -> bool {
    if rects.is_empty() {
        return true;
    }

    if board.passes == 1 {
        return true;
    }

    let live_count = board.live_mask.popcount();
    if live_count <= 12 {
        return true;
    }

    let my_score = board.my_mask.popcount() as i32;
    let opp_score = board.opp_mask.popcount() as i32;
    let score_diff = my_score - opp_score;
    if score_diff >= 20 {
        return true;
    }

    // In opening/midgame, PASS should only be considered when the position is
    // already cramped; otherwise the heuristic is too eager to skip playable moves.
    if live_count > 20 {
        return false;
    }

    let all_moves_look_bad = rects.iter().all(|r| {
        let mv = r.to_move();
        let (_sd, rec, fresh, live, _own, area) = board.action_score(mv);
        rec == 0 && fresh == 0 && live <= 2 && area <= 4
    });
    if all_moves_look_bad {
        return true;
    }

    false
}

fn root_reply_score(child: &Board, use_mquality: bool, game_data: Option<&GameData>) -> i32 {
    let replies = generate_rectangles(&child.values);
    if replies.is_empty() {
        return 0;
    }

    let phase = MoveQualityTable::phase_for_position(child.live_mask.popcount(), replies.len());
    let mut best_score = 0;

    for reply in replies.iter() {
        let mv = reply.to_move();
        let features = compute_move_features(child, mv);
        let mut score = static_move_score(&features);

        if use_mquality {
            if let Some(mquality_bonus) = game_data
                .and_then(|gd| gd.mquality.as_ref())
                .map(|mq| {
                    let bucket = MoveQualityTable::score_bucket(score);
                    mq.bonus(reply.id as usize, phase, bucket) as i32
                })
            {
                score += mquality_bonus;
            }
        }

        best_score = best_score.max(score);
    }

    if should_consider_pass(child, &replies) {
        let my_score = child.my_mask.popcount() as i32;
        let opp_score = child.opp_mask.popcount() as i32;
                let pass_score = if child.passes == 1 && my_score > opp_score {
                    i32::MAX - 100
                } else {
                    -250
                };
                best_score = best_score.max(pass_score);
    }

    best_score.max(0)
}

impl Search {
    pub fn order_moves_with_pass(
        &self,
        board: &Board,
        rects: &[RectInfo],
        tt_move: Option<Move>,
        ply: u8,
        threat_detected: bool,
        is_root: bool,
        prev_move: Option<Move>,
    ) -> Vec<Move> {
        if !self.config.use_ordering {
            let mut ordered: Vec<Move> = rects.iter().map(|r| r.to_move()).collect();
            if should_consider_pass(board, rects) {
                ordered.push(PASS);
            }
            return ordered;
        }

        let mut scored: Vec<(i32, Move)> = rects
            .iter()
            .map(|r| {
                let mv = r.to_move();
                let score = if Some(mv) == tt_move {
                    i32::MAX
                } else {
                    let features = compute_move_features(board, mv);
                    let mut score = static_move_score(&features);
                    let phase = MoveQualityTable::phase_for_position(board.live_mask.popcount(), rects.len());
                    let tactical_priority = self.tactical_priority(board, mv);

                    if self.config.use_mquality {
                        if let Some(mquality_bonus) = self
                            .game_data
                            .as_ref()
                            .and_then(|gd| gd.mquality.as_ref())
                            .map(|mq| {
                                let bucket = MoveQualityTable::score_bucket(score);
                                let r_id = fixed_rect_id(r.r1, r.c1, r.r2, r.c2);
                                mq.bonus(r_id as usize, phase, bucket) as i32
                            })
                        {
                            score += mquality_bonus;
                        }
                    }

                    score += tactical_priority * 4;

                    if threat_detected {
                        score += features.recaptured * 30 + features.fresh * 4;
                    }

                    let board_live = board.live_mask.popcount() as i32;
                    if board_live <= 12 {
                        score += endgame_move_bonus(board, mv, rects, self.game_data.as_ref());
                    } else if board_live < 20 {
                        score += (20 - board_live) * (features.recaptured * 3 + features.corner + features.edge);
                        score += features.area * 2;
                        if tactical_priority > 0 {
                            score += tactical_priority / 2;
                        }
                    } else if board_live > 32 {
                        score += opening_move_bonus(board, mv, rects, score, self.config.use_mquality, self.game_data.as_ref());
                    } else {
                        score += midgame_move_bonus(board, mv, rects, self.game_data.as_ref());
                    }

                    let history_bonus = self.history_score(ply, mv);
                    if features.recaptured > 0 {
                        score += history_bonus * 4;
                        score += 500;
                    } else {
                        score += history_bonus * 2;
                    }

                    // Static Exchange Evaluation bonus/penalty
                    let see_score = self.see(board, mv, rects);
                    score += see_score * 50;

                    if !is_root && ply < 64 {
                        if Some(mv) == self.killer_moves[ply as usize][0] {
                            score += 10000;
                        } else if Some(mv) == self.killer_moves[ply as usize][1] {
                            score += 5000;
                        }
                    }

                    if let Some(pmv) = prev_move {
                        if pmv != PASS {
                            let prev_id = fixed_rect_id(pmv.0, pmv.1, pmv.2, pmv.3) as usize;
                            let action_id = fixed_rect_id(r.r1, r.c1, r.r2, r.c2);
                            if self.counter_moves[prev_id] == action_id {
                                score += 8000;
                            }
                        }
                    }
                    score
                };
                (score, mv)
            })
            .collect();

        // Add PASS if we should consider it
        if should_consider_pass(board, rects) {
            let pass_move = PASS;
            let pass_score = if Some(pass_move) == tt_move {
                i32::MAX
            } else {
                let my_score = board.my_mask.popcount() as i32;
                let opp_score = board.opp_mask.popcount() as i32;
                        if board.passes == 1 && my_score > opp_score {
                            // Winning pass! Very high priority.
                            i32::MAX - 100
                        } else {
                            // Normal pass should stay below ordinary claim moves.
                            -250
                        }
                    };
            scored.push((pass_score, pass_move));
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        if is_root {
            let root_live = board.live_mask.popcount();
            let root_lead = board.my_mask.popcount() as i32 - board.opp_mask.popcount() as i32;
            let root_legal = rects.len();
            let defensive_mode = root_live <= DEFENSIVE_LIVE_THRESHOLD && root_lead >= DEFENSIVE_LEAD_THRESHOLD;
            let bonus_limit = scored.len().min(ROOT_REPLY_TOP_K);
            let reply_weight = if root_legal <= 10 || root_live <= 35 {
                0.75
            } else if defensive_mode {
                0.35
            } else {
                0.15
            };

            for (score, mv) in scored.iter_mut().take(bonus_limit) {
                if *score >= i32::MAX - 1000 {
                    continue;
                }

                let child = board.apply_action(*mv);
                let reply_score = root_reply_score(&child, self.config.use_mquality, self.game_data.as_ref()) as f32;
                let child_eval = child.lightweight_evaluate_with_weights(&self.weights).max(0.0);
                let eval_weight = if defensive_mode { 1.0 } else { 0.15 };
                let mut adjustment = -(reply_score * reply_weight) - (child_eval * eval_weight);

                if defensive_mode {
                    let child_pressure = (child.my_mask.popcount() as i32 - child.opp_mask.popcount() as i32).max(0) as f32;
                    adjustment -= child_pressure * 0.25;
                }

                *score += adjustment.round() as i32;
            }

            scored.sort_by(|a, b| b.0.cmp(&a.0));
        }

        if self.config.use_second_bonus && is_root && matches!(self.side, GameSide::Second) {
            let bonus_limit = scored.len().min(8); // ROOT_SECOND_BONUS_TOP_K is 8
            for (score, mv) in scored.iter_mut().take(bonus_limit) {
                if *score >= i32::MAX - 1000 {
                    continue;
                }
                let child = board.apply_action(*mv);
                let side_bonus = crate::side::second::SecondSideTuning::default().root_move_bonus(
                    board,
                    *mv,
                    &child,
                    &self.weights,
                    self.game_data.as_ref(),
                );
                *score += side_bonus.round() as i32;
            }
            scored.sort_by(|a, b| b.0.cmp(&a.0));
        }

        scored.into_iter().map(|(_, m)| m).collect()
    }
}
