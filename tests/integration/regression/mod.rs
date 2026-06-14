mod positions;

use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};

#[test]
fn test_regression_positions() {
    let positions = positions::get_regression_positions();

    for pos in positions {
        let board = Board::from_parts(pos.values, pos.owners, pos.player, 0);

        let mut search = Search::new(SearchConfig {
            time_budget_ms: 50,
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
        });

        let result = search.think(&board);
        assert_eq!(
            result.action, pos.expected_move,
            "Failed regression test '{}': expected move {:?}, bot chose {:?}",
            pos.name, pos.expected_move, result.action
        );
    }
}
