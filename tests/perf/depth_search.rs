use std::time::Instant;
use mushroom_bot::board::Board;
use mushroom_bot::search::{Search, SearchConfig};

fn create_midgame_board() -> Board {
    // A realistic midgame board with some live mushrooms and 0 cell owners initially
    let rows: Vec<String> = vec![
        "10020030040000000".to_string(),
        "02003004005000000".to_string(),
        "00300400500600000".to_string(),
        "00040050060070000".to_string(),
        "00005006007008000".to_string(),
        "00000000000000000".to_string(),
        "00000000000000000".to_string(),
        "00000000000000000".to_string(),
        "00000000000000000".to_string(),
        "00000000000000000".to_string(),
    ];
    Board::from_rows(&rows)
}

#[test]
fn test_depth_search_scaling() {
    let board = create_midgame_board();
    
    // Budgets to test in milliseconds
    let budgets = [50u64, 150u64, 400u64, 1000u64];
    
    println!("\n=== Bot Depth Search Scaling Performance ===");
    println!("{:<12} | {:<8} | {:<12} | {:<12} | {:<10}", "Budget (ms)", "Depth", "Nodes", "Elapsed (ms)", "NPS");
    println!("-----------------------------------------------------------------");
    
    let mut last_depth = 0;
    
    for &budget in &budgets {
        let mut search = Search::new(SearchConfig {
            time_budget_ms: budget,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: true,
            use_mcts: false,
            use_qsearch: true,
            use_lmr: true,
            use_futility: true,
            use_mquality: false,
            use_exact_endgame: true,
            use_nmp: true,
            use_singular_extension: true,
            use_mtd: false,
        });
        
        let start = Instant::now();
        let result = search.think(&board);
        let elapsed = start.elapsed().as_millis() as u64;
        
        let nps = if elapsed > 0 {
            (result.nodes * 1000) / elapsed
        } else {
            0
        };
        
        println!("{:<12} | {:<8} | {:<12} | {:<12} | {:<10} | value={}", 
            budget, 
            result.depth, 
            result.nodes, 
            elapsed, 
            nps,
            result.value
        );
        
        // Assertions:
        // 1. Reached depth must be at least 1
        assert!(result.depth >= 1, "Should reach at least depth 1");
        
        // 2. Depth reached with larger budget should be >= depth with smaller budget
        assert!(result.depth >= last_depth, "Depth should scale with budget ({} vs {})", result.depth, last_depth);
        last_depth = result.depth;
        
        // 3. Elapsed time should not exceed budget + safety margin
        assert!(elapsed <= budget + 250, "Search timed out significantly: elapsed {}ms for budget {}ms", elapsed, budget);
    }
    println!("============================================");
}
