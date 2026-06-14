use mushroom_bot::board::Board;
use mushroom_bot::dataloader::EvalWeights;
use mushroom_bot::types::*;

fn create_asymmetric_board() -> Board {
    let mut values = [0u8; N_CELLS];
    let mut owners = [0i8; N_CELLS];
    // Player FIRST owns some cells
    owners[cell_index(0, 0)] = FIRST;
    owners[cell_index(0, 1)] = FIRST;
    owners[cell_index(0, 2)] = FIRST;
    // Player SECOND owns some cells
    owners[cell_index(1, 0)] = SECOND;
    owners[cell_index(1, 1)] = SECOND;
    
    // Set some live mushrooms
    values[cell_index(2, 0)] = 5;
    values[cell_index(2, 1)] = 5;

    Board::from_parts(values, owners, FIRST, 0)
}

#[test]
fn test_fast_eval_symmetry() {
    let board_first = create_asymmetric_board();
    // Swap player to SECOND, but keep cell ownership identical
    let board_second = Board::from_parts(board_first.values, board_first.owners, SECOND, 0);

    let eval_first = board_first.fast_eval();
    let eval_second = board_second.fast_eval();

    assert_eq!(eval_first, 1.0); // 3 (my) - 2 (opp)
    assert_eq!(eval_second, -1.0); // 2 (my) - 3 (opp)
    assert_eq!(eval_first, -eval_second, "fast_eval must be symmetric");
}

#[test]
fn test_terminal_score_symmetry() {
    let board_first = create_asymmetric_board();
    let board_second = Board::from_parts(board_first.values, board_first.owners, SECOND, 0);

    let score_first = board_first.terminal_score();
    let score_second = board_second.terminal_score();

    assert!(score_first > 1_000_000.0, "FIRST wins and should have positive terminal score");
    assert!(score_second < -1_000_000.0, "SECOND loses and should have negative terminal score");
    
    // Check exact diff: 1_000_000 + 1 (first) vs -1_000_000 - 1 (second)
    assert_eq!(score_first, 1_000_000.0 + 1.0);
    assert_eq!(score_second, -1_000_000.0 - 1.0);
}

#[test]
fn test_weight_sanity() {
    let board = create_asymmetric_board(); // 3 cells owned by FIRST, 2 by SECOND

    let weights_default = EvalWeights::default();
    let mut weights_high_territory = EvalWeights::default();
    weights_high_territory.territory = weights_default.territory * 2.0;

    let eval_default = board.lightweight_evaluate_with_weights(&weights_default);
    let eval_high_territory = board.lightweight_evaluate_with_weights(&weights_high_territory);

    assert!(
        eval_high_territory > eval_default,
        "Increasing territory weight must increase evaluation score since player has net positive territory"
    );
}
