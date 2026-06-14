use mushroom_bot::board::Board;
use mushroom_bot::dataloader::{load_data_bin, EvalWeights, GameData};
use mushroom_bot::mcts::root_mcts_search;
use mushroom_bot::movegen::generate_rectangles;
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::side::GameSide;
use mushroom_bot::timeman::SearchPhase;
use mushroom_bot::types::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BotId(pub usize);

pub type BotFn = fn(&Board, u64) -> (i8, i8, i8, i8);

fn load_weights() -> (EvalWeights, Option<GameData>) {
    for path in &["data/data.bin", "data.bin"] {
        if let Some(gd) = load_data_bin(path) {
            return (gd.weights, Some(gd));
        }
    }
    (EvalWeights::default(), Some(GameData::default()))
}

fn make_search(
    board: &Board,
    budget: u64,
    config: SearchConfig,
    weights: EvalWeights,
    game_data: Option<GameData>,
    use_side_tuning: bool,
) -> Search {
    let mut search = if let Some(gd) = game_data {
        Search::with_game_data(SearchConfig { time_budget_ms: budget.max(1), ..config }, gd)
    } else {
        Search::with_weights(SearchConfig { time_budget_ms: budget.max(1), ..config }, weights)
    };

    if use_side_tuning {
        let side = GameSide::from_player(board.player);
        let tuned = side.tuning().search_config(config, SearchPhase::MidgameFull);
        search.config = SearchConfig {
            time_budget_ms: budget.max(1),
            ..tuned
        };
        search.set_side(side);
    }

    search
}

fn search_move(board: &Board, budget: u64, config: SearchConfig, use_side_tuning: bool) -> (i8, i8, i8, i8) {
    let (weights, game_data) = load_weights();
    let mut search = make_search(board, budget, config, weights, game_data, use_side_tuning);
    let result = search.think(board);
    if board.is_legal_action(result.action) {
        result.action
    } else {
        PASS
    }
}

fn greedy_balanced_choice(board: &Board) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    rects
        .iter()
        .max_by_key(|r| board.action_score(r.to_move()))
        .map(|r| r.to_move())
        .unwrap_or(PASS)
}

fn reply_aware_choice(board: &Board, weight: f64) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }

    rects
        .iter()
        .map(|r| {
            let mv = r.to_move();
            let immediate = board.action_score(mv).0 as f64;
            let next = board.apply_action(mv);
            let replies = generate_rectangles(&next.values);
            let reply = replies
                .iter()
                .map(|rr| next.action_score(rr.to_move()).0)
                .max()
                .unwrap_or(0);
            let penalty = (replies.len().min(16) as f64) * 0.03;
            (immediate - weight * reply as f64 - penalty, mv)
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|s| s.1)
        .unwrap_or(PASS)
}

fn policy_choice(board: &Board, budget: u64) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }

    let lookahead_budget = budget.max(10);
    let actions = rects.len() + 1;

    if actions <= 6 && lookahead_budget >= 30 {
        if let Some(mv) = try_policy_minimax(board, lookahead_budget.min(200)) {
            let scored_mv = reply_aware_score(board, mv, 0.80);
            let heuristic = reply_aware_choice(board, 0.80);
            let scored_heuristic = reply_aware_score(board, heuristic, 0.80);
            if scored_mv >= scored_heuristic {
                return mv;
            }
        }
    }

    heuristic_choice(board, lookahead_budget)
}

fn try_policy_minimax(board: &Board, budget: u64) -> Option<(i8, i8, i8, i8)> {
    let (weights, game_data) = load_weights();
    let config = SearchConfig {
        time_budget_ms: budget.max(1),
        use_tt: true,
        use_ordering: true,
        use_second_bonus: false,
        use_aspiration: false,
        use_mcts: false,
        use_qsearch: false,
        use_lmr: false,
        use_futility: false,
        use_mquality: false,
        use_exact_endgame: true,
        use_nmp: false,
        use_singular_extension: false,
        use_mtd: false,
    };

    let mut search = make_search(board, budget, config, weights, game_data, false);
    let result = search.think(board);
    if board.is_legal_action(result.action) {
        Some(result.action)
    } else {
        None
    }
}

fn reply_aware_score(board: &Board, mv: (i8, i8, i8, i8), weight: f64) -> f64 {
    let immediate = board.action_score(mv).0 as f64;
    if mv == PASS {
        return immediate;
    }

    let next = board.apply_action(mv);
    let replies = generate_rectangles(&next.values);
    let reply = replies
        .iter()
        .map(|rr| next.action_score(rr.to_move()).0)
        .max()
        .unwrap_or(0);
    let penalty = (replies.len().min(16) as f64) * 0.03;
    immediate - weight * reply as f64 - penalty
}

fn heuristic_choice(board: &Board, budget: u64) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }

    let top_k = (budget / 10).max(1) as usize;
    let top_k = top_k.min(rects.len()).min(9);

    let mut scored: Vec<(f64, (i8, i8, i8, i8))> = Vec::with_capacity(top_k);
    for (idx, rect) in rects.iter().enumerate().take(top_k) {
        let mv = rect.to_move();
        let adjusted = if idx < 4 {
            reply_recovery_score(board, mv, 0.80)
        } else {
            reply_aware_score(board, mv, 0.80)
        };
        scored.push((adjusted, mv));
    }

    scored
        .into_iter()
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|s| s.1)
        .unwrap_or_else(|| rects.first().map(|r| r.to_move()).unwrap_or(PASS))
}

fn reply_recovery_score(board: &Board, mv: (i8, i8, i8, i8), weight: f64) -> f64 {
    let immediate = board.action_score(mv).0 as f64;
    if mv == PASS {
        return immediate;
    }

    let next = board.apply_action(mv);
    let replies = generate_rectangles(&next.values);
    if replies.is_empty() {
        return immediate;
    }

    let mut reply_scores: Vec<(i32, (i8, i8, i8, i8))> = replies
        .iter()
        .map(|r| (next.action_score(r.to_move()).0, r.to_move()))
        .collect();
    reply_scores.sort_by_key(|r| -r.0);
    reply_scores.truncate(2);

    let mut worst_adjusted = 0i32;
    for (reply_score, reply) in &reply_scores {
        let after = next.apply_action(*reply);
        let counters = generate_rectangles(&after.values);
        let counter = counters
            .iter()
            .map(|r| after.action_score(r.to_move()).0)
            .max()
            .unwrap_or(0);
        let adjusted_reply = reply_score - (counter as f64 * 0.35) as i32;
        if adjusted_reply > worst_adjusted {
            worst_adjusted = adjusted_reply;
        }
    }

    let mobility_penalty = (replies.len().min(16) as f64) * 0.03;
    immediate - weight * worst_adjusted as f64 - mobility_penalty
}

// === Bot 1: agent_bot style ===
pub fn bot_agent(board: &Board, budget: u64) -> (i8, i8, i8, i8) {
    search_move(
        board,
        budget,
        SearchConfig {
            time_budget_ms: budget.max(1),
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: true,
            use_mcts: false,
            use_qsearch: true,
            use_lmr: true,
            use_futility: true,
            use_mquality: true,
            use_exact_endgame: true,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        },
        true,
    )
}

// === Bot 2: cordyceps_bot style ===
pub fn bot_cordyceps(board: &Board, budget: u64) -> (i8, i8, i8, i8) {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return PASS;
    }

    let live = board.live_mask.popcount();
    if live <= 12 || rects.len() <= 10 {
        return search_move(
            board,
            budget.max(10),
            SearchConfig {
                time_budget_ms: budget.max(10),
                use_tt: true,
                use_ordering: true,
                use_second_bonus: false,
                use_aspiration: true,
                use_mcts: false,
                use_qsearch: false,
                use_lmr: false,
                use_futility: false,
                use_mquality: false,
                use_exact_endgame: true,
                use_nmp: false,
                use_singular_extension: false,
                use_mtd: false,
            },
            true,
        );
    }

    if rects.len() <= 12 {
        let candidates: Vec<Move> = rects.iter().map(|r| r.to_move()).collect();
        let (weights, game_data) = load_weights();
        if let Some(result) = root_mcts_search(
            board,
            &candidates,
            &weights,
            game_data.as_ref(),
            budget.max(50),
        ) {
            if board.is_legal_action(result.action) {
                return result.action;
            }
        }
    }

    search_move(
        board,
        budget,
        SearchConfig {
            time_budget_ms: budget.max(1),
            use_tt: true,
            use_ordering: true,
            use_second_bonus: false,
            use_aspiration: true,
            use_mcts: false,
            use_qsearch: true,
            use_lmr: true,
            use_futility: true,
            use_mquality: true,
            use_exact_endgame: false,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        },
        true,
    )
}

// === Bot 3: finding_bot style ===
pub fn bot_finding(board: &Board, budget: u64) -> (i8, i8, i8, i8) {
    let weights = EvalWeights {
        territory: 148.0,
        safe_territory: 211.0,
        vulnerability: -9.0,
        steal_potential: 39.0,
        mobility: 20.0,
        connectivity: 19.0,
        corner_bonus: 18.0,
        edge_bonus: 3.0,
    };

    let config = SearchConfig {
        time_budget_ms: budget.max(1),
        use_tt: true,
        use_ordering: true,
        use_second_bonus: true,
        use_aspiration: true,
        use_mcts: false,
        use_qsearch: true,
        use_lmr: true,
        use_futility: true,
        use_mquality: true,
        use_exact_endgame: true,
        use_nmp: false,
        use_singular_extension: false,
        use_mtd: false,
    };

    let mut search = Search::with_weights(config, weights);
    search.set_side(GameSide::from_player(board.player));
    search.config = GameSide::from_player(board.player)
        .tuning()
        .search_config(config, SearchPhase::MidgameFull);

    let result = search.think(board);
    if board.is_legal_action(result.action) {
        result.action
    } else {
        PASS
    }
}

// === Bot 4: policy_bot style ===
pub fn bot_policy(board: &Board, budget: u64) -> (i8, i8, i8, i8) {
    policy_choice(board, budget)
}

pub fn bot_list() -> Vec<(&'static str, BotFn)> {
    vec![
        ("agent", bot_agent as BotFn),
        ("cordyceps", bot_cordyceps as BotFn),
        ("finding", bot_finding as BotFn),
        ("policy", bot_policy as BotFn),
    ]
}
