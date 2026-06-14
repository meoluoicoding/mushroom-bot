use std::time::Instant;
use mushroom_bot::board::Board;
use mushroom_bot::movegen::generate_rectangles;
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::types::*;
use crate::data::PolicyData;

pub fn choose_action(board: &Board, my_time_ms: u64, data: &PolicyData) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() { return PASS; }

    let _start = Instant::now();
    let budget = turn_budget(board, my_time_ms);
    let actions = rects.len() + 1;

    if actions <= 6 && budget >= 30 {
        if let Some(mv) = try_minimax(board, budget.min(200), data) {
            let (adj, ..) = reply_aware_rank(board, mv, 0.80);
            let heuristic = heuristic_choice(board, budget);
            let (h_adj, ..) = reply_aware_rank(board, heuristic, 0.80);
            if adj >= h_adj { return mv; }
        }
    }

    heuristic_choice(board, budget)
}

fn turn_budget(board: &Board, my_time_ms: u64) -> u64 {
    let usable = my_time_ms.saturating_sub(500);
    let live = board.live_mask.popcount() as u64;
    let est_turns = (live / 10).max(2).min(80);
    (usable / est_turns).clamp(10, 100)
}

fn try_minimax(board: &Board, budget_ms: u64, data: &PolicyData) -> Option<(i8, i8, i8, i8)> {
    let mut search = if let Some(ref gd) = data.game_data {
        Search::with_game_data(SearchConfig {
            time_budget_ms: budget_ms.max(1),
            use_tt: true, use_ordering: true, use_second_bonus: false,
            use_aspiration: false, use_mcts: false, use_qsearch: false,
            use_lmr: false, use_futility: false, use_mquality: false,
            use_exact_endgame: true, use_nmp: false, use_singular_extension: false, use_mtd: false,
        }, gd.clone())
    } else {
        Search::with_weights(SearchConfig {
            time_budget_ms: budget_ms.max(1),
            use_tt: true, use_ordering: true, use_second_bonus: false,
            use_aspiration: false, use_mcts: false, use_qsearch: false,
            use_lmr: false, use_futility: false, use_mquality: false,
            use_exact_endgame: true, use_nmp: false, use_singular_extension: false, use_mtd: false,
        }, data.weights)
    };
    let result = search.think(board);
    if board.is_legal_action(result.action) { Some(result.action) } else { None }
}

fn reply_aware_rank(board: &Board, mv: (i8, i8, i8, i8), weight: f64) -> (f64, i32, i32, i32, i32, i32, i32) {
    let immediate = board.action_score(mv);
    if mv == PASS {
        return (immediate.0 as f64, immediate.0, immediate.1, immediate.2, immediate.3, immediate.4, immediate.5);
    }
    let next = board.apply_action(mv);
    let replies = generate_rectangles(&next.values);
    if replies.is_empty() {
        return (immediate.0 as f64, immediate.0, immediate.1, immediate.2, immediate.3, immediate.4, immediate.5);
    }
    let reply_score = replies.iter()
        .map(|r| next.action_score(r.to_move()).0)
        .max().unwrap_or(0);
    let mobility_penalty = (replies.len().min(16) as f64) * 0.03;
    let adjusted = immediate.0 as f64 - weight * reply_score as f64 - mobility_penalty;
    (adjusted, immediate.0, immediate.1, immediate.2, immediate.3, immediate.4, immediate.5)
}

fn heuristic_choice(board: &Board, budget_ms: u64) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() { return PASS; }

    let start = Instant::now();
    let per_candidate = 10u64;
    let lookahead = (budget_ms / per_candidate).max(1) as usize;
    let lookahead = lookahead.min(rects.len()).min(9);

    let mut scored: Vec<(f64, (i8, i8, i8, i8))> = Vec::with_capacity(lookahead);

    for (i, r) in rects.iter().enumerate().take(lookahead) {
        if start.elapsed().as_millis() as u64 >= budget_ms { break; }
        let mv = r.to_move();

        // Reply recovery for top candidates
        let (adjusted, ..) = if i < 4 {
            reply_recovery_rank(board, mv, 0.80)
        } else {
            reply_aware_rank(board, mv, 0.80)
        };
        scored.push((adjusted, mv));
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.first().map(|s| s.1).unwrap_or_else(|| {
        rects.first().map(|r| r.to_move()).unwrap_or(PASS)
    })
}

fn reply_recovery_rank(board: &Board, mv: (i8, i8, i8, i8), weight: f64) -> (f64, i32, i32, i32, i32, i32, i32) {
    let immediate = board.action_score(mv);
    if mv == PASS {
        return (immediate.0 as f64, immediate.0, immediate.1, immediate.2, immediate.3, immediate.4, immediate.5);
    }
    let next = board.apply_action(mv);
    let replies = generate_rectangles(&next.values);
    if replies.is_empty() {
        return (immediate.0 as f64, immediate.0, immediate.1, immediate.2, immediate.3, immediate.4, immediate.5);
    }

    // Get top 2 opposing replies
    let mut reply_scores: Vec<(i32, (i8, i8, i8, i8))> = replies.iter()
        .map(|r| (next.action_score(r.to_move()).0, r.to_move()))
        .collect();
    reply_scores.sort_by_key(|r| -r.0);
    reply_scores.truncate(2);

    let mut worst_adjusted = 0i32;
    for (reply_score, reply) in &reply_scores {
        let after = next.apply_action(*reply);
        let counters = generate_rectangles(&after.values);
        let counter = counters.iter().map(|r| after.action_score(r.to_move()).0).max().unwrap_or(0);
        let adjusted_reply = reply_score - (counter as f64 * 0.35) as i32;
        if adjusted_reply > worst_adjusted { worst_adjusted = adjusted_reply; }
    }

    let mobility_penalty = (replies.len().min(16) as f64) * 0.03;
    let adjusted = immediate.0 as f64 - weight * worst_adjusted as f64 - mobility_penalty;
    (adjusted, immediate.0, immediate.1, immediate.2, immediate.3, immediate.4, immediate.5)
}
