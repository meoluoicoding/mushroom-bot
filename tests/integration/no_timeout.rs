use std::time::Instant;
use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};

fn create_full_board() -> Board {
    let rows: Vec<String> = vec![
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
        "12345678912345678".to_string(),
    ];
    Board::from_rows(&rows)
}

#[test]
fn test_search_respects_time_budget() {
    let board = create_full_board();
    
    // Set budget to 40ms
    let budget_ms = 40;
    let mut search = Search::new(SearchConfig {
        time_budget_ms: budget_ms,
        use_tt: true,
        use_ordering: true,
        use_second_bonus: true,
        use_aspiration: true,
        use_mcts: false,
        use_qsearch: true,
        use_lmr: true,
        use_futility: true,
        use_mquality: false,
        use_exact_endgame: false,
        use_nmp: true,
        use_singular_extension: false,
        use_mtd: false,
    });

    let start = Instant::now();
    let _result = search.think(&board);
    let elapsed = start.elapsed().as_millis();

    // Check that we didn't exceed budget by too much (allow 20ms buffer for OS/thread scheduler jitter)
    assert!(
        elapsed < (budget_ms + 20) as u128,
        "Search took {}ms, which significantly exceeded the budget of {}ms",
        elapsed,
        budget_ms
    );
}
