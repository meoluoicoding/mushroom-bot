use std::io::BufRead;

use mushroom_bot::board::Board;
use mushroom_bot::types::*;

use crate::context::EvalContext;
use crate::keys::{
    area_key, corner_key, defensive_key, defensive_when_losing_key, edge_key, fresh_key,
    live_cell_key, net_area_key, position_key, recapture_key, reply_aware_key,
};
use crate::rng::ZooRng;
use crate::search::{
    minimax_choice, mcts_choice, random_top_choice, search_engine_choice,
};
use crate::styles::{
    adaptive_hybrid_choice, endgame_expert_choice, greedy_balanced_choice,
    mixed_tactical_choice, pass_abuser_choice, pass_safe_choice,
};
use crate::utils::{format_move, legal_rectangles, parse_move, parse_u64, stable_seed};

pub struct ZooProtocolBot {
    pub mode: String,
    pub rng: ZooRng,
    pub state: Option<Board>,
    pub my_player: i8,
    pub my_time_left_ms: Option<u64>,
    pub opp_time_left_ms: Option<u64>,
    pub ctx: EvalContext,
}

impl ZooProtocolBot {
    pub fn new(mode: String, seed: u64) -> Self {
        Self {
            mode,
            rng: ZooRng::new(seed),
            state: None,
            my_player: FIRST,
            my_time_left_ms: None,
            opp_time_left_ms: None,
            ctx: EvalContext::load(),
        }
    }

    pub fn handle_command(&mut self, line: &str, input_stream: &mut dyn BufRead) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "READY" => {
                self.my_player = if parts.get(1).copied() == Some("SECOND") {
                    SECOND
                } else {
                    FIRST
                };
                Some("OK".to_string())
            }
            "INIT" => {
                let mut rows: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                while rows.len() < ROWS {
                    let mut buf = String::new();
                    if input_stream.read_line(&mut buf).ok()? == 0 {
                        break;
                    }
                    let trimmed = buf.trim();
                    if !trimmed.is_empty() {
                        rows.extend(trimmed.split_whitespace().map(|s| s.to_string()));
                    }
                }
                rows.truncate(ROWS);
                self.state = Some(Board::from_rows(&rows));
                self.rng.reseed(stable_seed(&self.mode, &rows, 42));
                None
            }
            "TIME" => {
                let board = match &self.state {
                    Some(state) if !state.is_terminal() => state.clone(),
                    _ => return Some(format_move(PASS)),
                };
                self.my_time_left_ms = parse_u64(parts.get(1));
                self.opp_time_left_ms = parse_u64(parts.get(2));
                let budget_ms = self.budget_ms();
                let mv = choose_move(&self.mode, &board, budget_ms, &mut self.rng, &self.ctx);
                let mv = if board.is_legal_action(mv) { mv } else { PASS };
                self.state = Some(board.apply_action(mv));
                Some(format_move(mv))
            }
            "OPP" => {
                if let Some(state) = &self.state {
                    if !state.is_terminal() && parts.len() >= 5 {
                        if let Some(mv) = parse_move(&parts[1..5]) {
                            if state.is_legal_action(mv) {
                                self.state = Some(state.apply_action(mv));
                            }
                        }
                    }
                }
                None
            }
            "FINISH" => {
                std::process::exit(0);
            }
            _ => None,
        }
    }

    fn budget_ms(&self) -> u64 {
        let usable = self.my_time_left_ms.unwrap_or(25).saturating_sub(300);
        if self.mode.starts_with("minimax") {
            return usable.saturating_div(5).clamp(200, 500);
        }
        if self.mode.starts_with("search_") || self.mode.starts_with("mcts_") {
            return usable.saturating_div(3).clamp(100, 2000);
        }
        usable.saturating_div(20).clamp(5, 60)
    }
}

fn choose_move(mode: &str, board: &Board, budget_ms: u64, rng: &mut ZooRng, ctx: &EvalContext) -> Move {
    let rects = legal_rectangles(board);
    if rects.is_empty() {
        return PASS;
    }

    match mode {
        "greedy_area" => rects
            .iter()
            .max_by_key(|r| area_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_recapture" => rects
            .iter()
            .max_by_key(|r| recapture_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_fresh" => rects
            .iter()
            .max_by_key(|r| fresh_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_edge" => rects
            .iter()
            .max_by_key(|r| edge_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_corner" => rects
            .iter()
            .max_by_key(|r| corner_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_net_area" => rects
            .iter()
            .max_by_key(|r| net_area_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_live_cell" => rects
            .iter()
            .max_by_key(|r| live_cell_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "greedy_position" => rects
            .iter()
            .max_by_key(|r| position_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "reply_aware" => rects
            .iter()
            .max_by_key(|r| reply_aware_key(board, r.to_move(), 0.80))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "reply_aware_strict" => rects
            .iter()
            .max_by_key(|r| reply_aware_key(board, r.to_move(), 1.10))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "reply_aware_light" => rects
            .iter()
            .max_by_key(|r| reply_aware_key(board, r.to_move(), 0.50))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "defensive_when_leading" => rects
            .iter()
            .max_by_key(|r| defensive_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "defensive_when_losing" => rects
            .iter()
            .max_by_key(|r| defensive_when_losing_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS),
        "pass_abuser" => pass_abuser_choice(board),
        "pass_safe" => pass_safe_choice(board, rng),
        "random_top_1" => random_top_choice(board, 1, rng),
        "random_top_3" => random_top_choice(board, 3, rng),
        "random_top_5" => random_top_choice(board, 5, rng),
        "random_top_7" => random_top_choice(board, 7, rng),
        "minimax_depth_1" => minimax_choice(board, 1, budget_ms, ctx),
        "minimax_depth_2" => minimax_choice(board, 2, budget_ms, ctx),
        "minimax_depth_3" => minimax_choice(board, 3, budget_ms, ctx),
        "minimax_depth_4" => minimax_choice(board, 4, budget_ms, ctx),
        "greedy_balanced" => greedy_balanced_choice(board),
        "mixed_tactical" => mixed_tactical_choice(board, budget_ms, rng, ctx),
        "endgame_expert" => endgame_expert_choice(board, budget_ms, rng, ctx),
        "search_shallow" => search_engine_choice(board, budget_ms.min(50), ctx, 2, false),
        "search_medium" => search_engine_choice(board, budget_ms.min(200), ctx, 4, false),
        "search_deep" => search_engine_choice(board, budget_ms, ctx, 6, false),
        "search_first_side" => search_engine_choice(board, budget_ms.min(100), ctx, 3, true),
        "mcts_light" => mcts_choice(board, budget_ms.min(80), ctx, 200),
        "mcts_deep" => mcts_choice(board, budget_ms, ctx, 800),
        "adaptive_hybrid" => adaptive_hybrid_choice(board, budget_ms, rng, ctx),
        _ => greedy_balanced_choice(board),
    }
}
