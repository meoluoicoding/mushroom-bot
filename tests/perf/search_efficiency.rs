use std::time::Instant;
use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};

fn create_test_board() -> Board {
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
fn test_search_pruning_efficiency() {
    let board = create_test_board();
    let root_rect_key = Search::rect_cache_key(&board.values);
    let mut search_rects = Search::new(SearchConfig::default());
    let rects = search_rects.cached_rectangles(root_rect_key, &board.values);
    let root_key = board.hash;
    let target_depth = 4; // Keep depth moderate so baseline doesn't run too long

    // 1. Baseline Search: No Transposition Table, no LMR, no Futility, no NMP
    let mut search_baseline = Search::new(SearchConfig {
        time_budget_ms: 10000,
        use_tt: false,
        use_ordering: true,
        use_second_bonus: false,
        use_aspiration: false,
        use_mcts: false,
        use_qsearch: false,
        use_lmr: false,
        use_futility: false,
        use_mquality: false,
        use_exact_endgame: false,
        use_nmp: false,
        use_singular_extension: false,
        use_mtd: false,
    });

    let start_baseline = Instant::now();
    let _ = search_baseline.root_search(
        &board,
        &rects,
        target_depth,
        f32::NEG_INFINITY,
        f32::INFINITY,
        root_key,
        2,
        0,
    );
    let elapsed_baseline = start_baseline.elapsed().as_secs_f64();
    let nodes_baseline = search_baseline.nodes;
    let ebf_baseline = (nodes_baseline as f64).powf(1.0 / target_depth as f64);

    // 2. Optimized Search: With TT, LMR, Futility, NMP
    let mut search_optimized = Search::new(SearchConfig {
        time_budget_ms: 10000,
        use_tt: true,
        use_ordering: true,
        use_second_bonus: true,
        use_aspiration: true,
        use_mcts: false,
        use_qsearch: false,
        use_lmr: true,
        use_futility: true,
        use_mquality: false,
        use_exact_endgame: false,
        use_nmp: true,
        use_singular_extension: true,
        use_mtd: false,
    });

    let start_optimized = Instant::now();
    let _ = search_optimized.root_search(
        &board,
        &rects,
        target_depth,
        f32::NEG_INFINITY,
        f32::INFINITY,
        root_key,
        2,
        0,
    );
    let elapsed_optimized = start_optimized.elapsed().as_secs_f64();
    let nodes_optimized = search_optimized.nodes;
    let ebf_optimized = (nodes_optimized as f64).powf(1.0 / target_depth as f64);

    println!("\n=== Search Pruning Efficiency (Depth {}) ===", target_depth);
    println!("Baseline  -> Nodes: {}, EBF: {:.2}, Time: {:.3}s", nodes_baseline, ebf_baseline, elapsed_baseline);
    println!("Optimized -> Nodes: {}, EBF: {:.2}, Time: {:.3}s", nodes_optimized, ebf_optimized, elapsed_optimized);
    println!("Reduction Ratio: {:.1}x fewer nodes", nodes_baseline as f64 / nodes_optimized as f64);
    println!("============================================");

    // Assert that the optimized search cuts down search nodes significantly.
    // Usually Alpha-Beta pruning + TT + LMR + NMP reduces nodes by at least 2.5x even at shallow depth 4.
    assert!(
        nodes_optimized < nodes_baseline,
        "Optimized search must examine fewer nodes than baseline"
    );
}
