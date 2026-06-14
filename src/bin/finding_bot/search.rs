use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::dataloader::{EvalWeights, GameData, load_data_bin};
use mushroom_bot::types::*;

pub static mut NODES: u64 = 0;

pub fn find_best_move(board: &Board, time_budget_ms: u64) -> (i8, i8, i8, i8) {
    unsafe { NODES = 0; }

    let (weights, game_data) = load_tuned_data();
    let config = SearchConfig {
        time_budget_ms: time_budget_ms.max(1),
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

    let mut search = if let Some(gd) = game_data {
        Search::with_game_data(config, gd)
    } else {
        Search::with_weights(config, weights)
    };

    let result = search.think(board);
    if board.is_legal_action(result.action) { result.action } else { PASS }
}

fn load_tuned_data() -> (EvalWeights, Option<GameData>) {
    for path in &["data/data.bin", "data.bin"] {
        if let Some(gd) = load_data_bin(path) { return (gd.weights, Some(gd)); }
    }
    // Optuna-tuned weights from main2.cpp p1a-finding-v1
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
    (weights, Some(GameData::default()))
}
