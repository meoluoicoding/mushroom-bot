use mushroom_bot::board::Board;
use mushroom_bot::types::*;

use crate::context::EvalContext;
use crate::keys::{
    defensive_key, recapture_key, reply_aware_key,
};
use crate::rng::ZooRng;
use crate::search::{
    minimax_choice, random_top_choice, search_engine_choice,
};
use crate::utils::{legal_rectangles, live_count};

pub fn pass_abuser_choice(board: &Board) -> Move {
    let rects = legal_rectangles(board);
    if rects.is_empty() {
        return PASS;
    }
    let opponent = mushroom_bot::types::opponent(board.player);
    let lead = board.score(board.player) - board.score(opponent);
    let live = live_count(board);
    if lead >= 4 && rects.len() <= 6 {
        return PASS;
    }
    if live <= 12 || rects.len() <= 2 {
        return PASS;
    }
    crate::search::greedy_balanced_choice(board)
}

pub fn pass_safe_choice(board: &Board, rng: &mut ZooRng) -> Move {
    let rects = legal_rectangles(board);
    if rects.is_empty() {
        return PASS;
    }
    let opponent = mushroom_bot::types::opponent(board.player);
    let lead = board.score(board.player) - board.score(opponent);
    let live = live_count(board);
    if lead >= 8 && live <= 18 {
        return PASS;
    }
    random_top_choice(board, 3, rng)
}

pub fn mixed_tactical_choice(
    board: &Board,
    budget_ms: u64,
    rng: &mut ZooRng,
    ctx: &EvalContext,
) -> Move {
    let rects = legal_rectangles(board);
    if rects.is_empty() {
        return PASS;
    }
    if rects.len() <= 3 {
        return rects
            .iter()
            .max_by_key(|r| recapture_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS);
    }
    let roll = rng.roll_f64();
    if roll < 0.35 {
        return rects
            .iter()
            .max_by_key(|r| reply_aware_key(board, r.to_move(), 0.80))
            .map(|r| r.to_move())
            .unwrap_or(PASS);
    }
    if roll < 0.70 {
        return rects
            .iter()
            .max_by_key(|r| defensive_key(board, r.to_move()))
            .map(|r| r.to_move())
            .unwrap_or(PASS);
    }
    minimax_choice(board, 2, budget_ms, ctx)
}

pub fn endgame_expert_choice(
    board: &Board,
    budget_ms: u64,
    rng: &mut ZooRng,
    ctx: &EvalContext,
) -> Move {
    if live_count(board) <= 20 {
        minimax_choice(board, 4, budget_ms, ctx)
    } else {
        random_top_choice(board, 5, rng)
    }
}

pub fn greedy_balanced_choice(board: &Board) -> Move {
    crate::search::greedy_balanced_choice(board)
}

pub fn adaptive_hybrid_choice(
    board: &Board,
    budget_ms: u64,
    rng: &mut ZooRng,
    ctx: &EvalContext,
) -> Move {
    let rects = legal_rectangles(board);
    if rects.is_empty() {
        return PASS;
    }

    let live = live_count(board);
    let num_rects = rects.len();
    let opponent = mushroom_bot::types::opponent(board.player);
    let lead = board.score(board.player) - board.score(opponent);

    if live <= 12 || num_rects <= 6 {
        return minimax_choice(board, 4, budget_ms, ctx);
    }

    if live <= 20 && budget_ms >= 200 {
        return minimax_choice(board, 3, budget_ms, ctx);
    }

    if lead <= -6 {
        return search_engine_choice(board, budget_ms, ctx, 4, false);
    }

    if num_rects <= 10 {
        return minimax_choice(board, 2, budget_ms, ctx);
    }

    if budget_ms >= 300 {
        return search_engine_choice(board, budget_ms / 2, ctx, 3, false);
    }

    if lead >= 6 {
        return pass_safe_choice(board, rng);
    }

    mixed_tactical_choice(board, budget_ms, rng, ctx)
}
