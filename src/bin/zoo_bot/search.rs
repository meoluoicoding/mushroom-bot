use std::cmp::Reverse;
use std::time::Instant;

use mushroom_bot::board::Board;
use mushroom_bot::mcts::root_mcts_search;
use mushroom_bot::movegen::generate_rectangles;
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::side::GameSide;
use mushroom_bot::types::*;

use crate::context::EvalContext;
use crate::keys::move_key;
use crate::rng::ZooRng;

pub fn greedy_balanced_choice(board: &Board) -> Move {
    let rects = generate_rectangles(&board.values);
    rects
        .iter()
        .max_by_key(|r| move_key(board, r.to_move()))
        .map(|r| r.to_move())
        .unwrap_or(PASS)
}

pub fn random_top_choice(board: &Board, top_k: usize, rng: &mut ZooRng) -> Move {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }
    let mut moves: Vec<Move> = rects.iter().map(|r| r.to_move()).collect();
    moves.sort_by_key(|&mv| Reverse(move_key(board, mv)));
    let top_k = top_k.max(1).min(moves.len());
    moves[rng.gen_range(top_k)]
}

pub fn minimax_choice(board: &Board, depth: u8, budget_ms: u64, ctx: &EvalContext) -> Move {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }
    let start = Instant::now();
    let mut ordered: Vec<Move> = rects.iter().map(|r| r.to_move()).collect();
    ordered.sort_by_key(|&mv| Reverse(move_key(board, mv)));

    let mut best_move = ordered[0];
    let mut best_value = f32::NEG_INFINITY;
    for mv in ordered {
        if start.elapsed().as_millis() as u64 >= budget_ms {
            break;
        }
        let child = board.apply_action(mv);
        let value = -negamax(
            &child,
            depth.saturating_sub(1),
            f32::NEG_INFINITY,
            f32::INFINITY,
            start,
            budget_ms,
            ctx,
        );
        if value > best_value {
            best_value = value;
            best_move = mv;
        }
    }

    best_move
}

pub fn negamax(
    board: &Board,
    depth: u8,
    mut alpha: f32,
    beta: f32,
    start: Instant,
    budget_ms: u64,
    ctx: &EvalContext,
) -> f32 {
    if board.is_terminal() {
        return board.terminal_score();
    }
    if depth == 0 || start.elapsed().as_millis() as u64 >= budget_ms {
        return board.evaluate(&ctx.weights, ctx.game_data.as_ref());
    }

    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        let child = board.apply_action(PASS);
        if child.is_terminal() {
            return board.terminal_score();
        }
        return -negamax(&child, depth - 1, -beta, -alpha, start, budget_ms, ctx);
    }

    let mut ordered: Vec<Move> = rects.iter().map(|r| r.to_move()).collect();
    ordered.sort_by_key(|&mv| Reverse(move_key(board, mv)));

    for mv in ordered {
        if start.elapsed().as_millis() as u64 >= budget_ms {
            break;
        }
        let child = board.apply_action(mv);
        let score = -negamax(&child, depth - 1, -beta, -alpha, start, budget_ms, ctx);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }

    alpha
}

pub fn search_engine_choice(
    board: &Board,
    budget_ms: u64,
    ctx: &EvalContext,
    _max_depth_hint: u8,
    use_side_tuning: bool,
) -> Move {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }

    let _live = board.live_mask.popcount();
    let mut config = SearchConfig {
        time_budget_ms: budget_ms,
        use_tt: true,
        use_ordering: true,
        use_second_bonus: use_side_tuning,
        use_aspiration: budget_ms >= 100,
        use_mcts: false,
        use_qsearch: budget_ms >= 50,
        use_lmr: budget_ms >= 100,
        use_futility: budget_ms >= 100,
        use_mquality: true,
        use_exact_endgame: true,
        use_nmp: false,
        use_singular_extension: false,
        use_mtd: false,
    };

    let side = if use_side_tuning {
        if board.player == FIRST {
            GameSide::First
        } else {
            GameSide::Second
        }
    } else {
        GameSide::First
    };

    let mut search = if let Some(ref gd) = ctx.game_data {
        Search::with_game_data(config, gd.clone())
    } else {
        Search::with_weights(config, ctx.weights)
    };

    if use_side_tuning {
        let tuning = side.tuning();
        config = tuning.search_config(config, mushroom_bot::timeman::SearchPhase::MidgameFull);
        search.config = config;
        search.set_side(side);
    }

    let result = search.think(board);
    if board.is_legal_action(result.action) {
        result.action
    } else {
        greedy_balanced_choice(board)
    }
}

pub fn mcts_choice(
    board: &Board,
    budget_ms: u64,
    ctx: &EvalContext,
    _max_iters: u32,
) -> Move {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }

    let candidates: Vec<Move> = rects.iter().map(|r| r.to_move()).collect();
    let result = root_mcts_search(
        board,
        &candidates,
        &ctx.weights,
        ctx.game_data.as_ref(),
        budget_ms,
    );

    match result {
        Some(r) if board.is_legal_action(r.action) => r.action,
        _ => greedy_balanced_choice(board),
    }
}
