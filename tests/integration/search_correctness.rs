use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::types::*;

fn create_puzzle_board() -> Board {
    let mut values = [0u8; N_CELLS];
    let owners = [0i8; N_CELLS];
    // Exactly one mushroom of value 10 at (0, 0)
    values[cell_index(0, 0)] = 10;
    Board::from_parts(values, owners, FIRST, 0)
}

#[test]
fn test_search_forced_win() {
    let board = create_puzzle_board();
    
    // Create search agent
    let mut search = Search::new(SearchConfig {
        time_budget_ms: 100,
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
    
    // The only winning move is capturing the single mushroom at (0,0)
    assert_eq!(result.action, (0, 0, 0, 0));
    assert!(result.value > 1_000_000.0, "Expected a terminal winning score");
}
