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
fn test_node_throughput() {
    let board = create_full_board();
    
    // We run with a generous time budget (e.g. 500ms) but config set to a target depth limit, 
    // or just let it run for exactly 300ms and see how many nodes it searched.
    let budget_ms = 300;
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
    let result = search.think(&board);
    let elapsed = start.elapsed();
    let elapsed_sec = elapsed.as_secs_f64();
    
    let nps = if elapsed_sec > 0.0 {
        result.nodes as f64 / elapsed_sec
    } else {
        0.0
    };

    println!(
        "\n=== Performance Baseline ===\nNodes searched: {}\nElapsed: {:.3}s\nNodes/sec: {:.0}\n============================",
        result.nodes,
        elapsed_sec,
        nps
    );

    // Sanity assertion to make sure search actually did some work
    assert!(result.nodes > 10, "Search should examine at least a few nodes");
}
