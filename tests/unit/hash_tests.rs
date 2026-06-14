use mushroom_bot::board::Board;
use mushroom_bot::tt::{hash_board, hash_update};
use mushroom_bot::types::*;

fn create_sample_board() -> Board {
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
fn test_hash_initial_state() {
    let board = create_sample_board();
    let expected = hash_board(&board.values, &board.owners, board.player, board.passes);
    assert_eq!(board.hash, expected);
}

#[test]
fn test_hash_incremental_update_after_moves() {
    let mut board = create_sample_board();
    
    // Perform a sequence of moves (both PASS and actual rectangle moves)
    let moves = vec![
        PASS,
        (0, 0, 0, 3), // row 0 cols 0..=3 sums to 10
        PASS,
        (1, 0, 1, 3), // row 1 cols 0..=3 sums to 10
    ];

    for mv in moves {
        let next_board = board.apply_action(mv);
        
        // Compute hash incrementally using hash_update directly
        let next_passes = if mv == PASS { board.passes + 1 } else { 0 };
        let computed_incremental = hash_update(
            board.hash,
            &board.values,
            &board.owners,
            mv,
            board.player,
            board.passes,
            next_passes,
        );

        // Compute hash from scratch on next_board properties
        let expected_scratch = hash_board(
            &next_board.values,
            &next_board.owners,
            next_board.player,
            next_board.passes,
        );

        assert_eq!(next_board.hash, expected_scratch, "Board struct hash must match full scratch computation");
        assert_eq!(computed_incremental, expected_scratch, "Incremental hash_update must match full scratch computation");

        board = next_board;
    }
}
