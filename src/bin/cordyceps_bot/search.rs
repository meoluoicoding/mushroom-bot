use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};
use crate::data::CordycepsData;
use crate::mcts::mcts_search;
use crate::movegen::generate_moves;

pub fn hybrid_search(board: &Board, my_time_ms: i32, _opp_time_ms: i32, data: &CordycepsData) -> (i8, i8, i8, i8) {
    let moves = generate_moves(board);
    if moves.is_empty() { return (-1, -1, -1, -1); }

    let live = board.live_mask.popcount() as i32;
    let mut tm = crate::timeman::TimeManager::new();
    tm.init(my_time_ms, live);

    fn make_search(config: SearchConfig, data: &CordycepsData) -> Search {
        if let Some(ref gd) = data.game_data {
            Search::with_game_data(config, gd.clone())
        } else {
            Search::with_weights(config, data.weights)
        }
    }

    match tm.phase {
        crate::timeman::GamePhase::EndgameExact => {
            let budget = tm.optimum_time_ms.max(10) as u64;
            let mut search = make_search(SearchConfig {
                time_budget_ms: budget, use_tt: true, use_ordering: true,
                use_second_bonus: false, use_aspiration: true, use_mcts: false,
                use_qsearch: false, use_lmr: false, use_futility: false,
                use_mquality: false, use_exact_endgame: true, use_nmp: false,
                use_singular_extension: false, use_mtd: false,
            }, data);
            let result = search.think(board);
            if board.is_legal_action(result.action) { return result.action; }
            moves[0]
        }
        crate::timeman::GamePhase::MidgameFull => {
            if moves.len() <= 12 {
                if let Some(mv) = mcts_search(board, tm.optimum_time_ms.max(50) as u64) {
                    if board.is_legal_action(mv) { return mv; }
                }
            }
            let budget = (tm.optimum_time_ms as u64 * 60 / 100).max(10);
            let mut search = make_search(SearchConfig {
                time_budget_ms: budget, use_tt: true, use_ordering: true,
                use_second_bonus: false, use_aspiration: true, use_mcts: false,
                use_qsearch: true, use_lmr: true, use_futility: true,
                use_mquality: true, use_exact_endgame: false, use_nmp: false,
                use_singular_extension: false, use_mtd: false,
            }, data);
            let result = search.think(board);
            if board.is_legal_action(result.action) { result.action } else { moves[0] }
        }
        crate::timeman::GamePhase::MidgameConserve => {
            let budget = tm.optimum_time_ms.max(10) as u64;
            let mut search = make_search(SearchConfig {
                time_budget_ms: budget, use_tt: true, use_ordering: true,
                use_second_bonus: false, use_aspiration: false, use_mcts: false,
                use_qsearch: false, use_lmr: false, use_futility: false,
                use_mquality: false, use_exact_endgame: false, use_nmp: false,
                use_singular_extension: false, use_mtd: false,
            }, data);
            let result = search.think(board);
            if board.is_legal_action(result.action) { result.action } else { moves[0] }
        }
        crate::timeman::GamePhase::Emergency => moves[0],
    }
}
