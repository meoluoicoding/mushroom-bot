use mushroom_bot::board::Board;
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
fn test_board_initial_state() {
    let board = create_sample_board();
    assert_eq!(board.player, FIRST);
    assert_eq!(board.passes, 0);
    assert!(!board.is_terminal());
    assert_eq!(board.my_mask.popcount(), 0);
    assert_eq!(board.opp_mask.popcount(), 0);
    assert_eq!(board.live_mask.popcount(), (ROWS * COLS) as u32);
}

#[test]
fn test_board_apply_pass() {
    let board = create_sample_board();
    let after_pass = board.apply_action(PASS);
    assert_eq!(after_pass.player, SECOND);
    assert_eq!(after_pass.passes, 1);
    assert!(!after_pass.is_terminal());

    let after_two_passes = after_pass.apply_action(PASS);
    assert_eq!(after_two_passes.player, FIRST);
    assert_eq!(after_two_passes.passes, 2);
    assert!(after_two_passes.is_terminal());
}

#[test]
fn test_board_apply_rect_move() {
    let board = create_sample_board();
    // Rectangle summing to 10 (e.g. cells at (0,0) and (0,1): 1 + 2 = 3, not 10.
    // Wait, let's find a valid rectangle that sums to 10 in our sample board.
    // Row 0 has: 1, 2, 3, 4. 1+2+3+4 = 10!
    // So columns 0 to 3 of row 0 is a valid rectangle.
    let mv = (0, 0, 0, 3);
    assert!(board.is_legal_action(mv));

    let next_board = board.apply_action(mv);
    assert_eq!(next_board.player, SECOND);
    assert_eq!(next_board.passes, 0);

    // After move, columns 0-3 on row 0 should be owned by FIRST (which is now opponent for SECOND player next_board)
    for c in 0..=3 {
        let idx = cell_index(0, c);
        assert_eq!(next_board.owners[idx], FIRST);
        assert_eq!(next_board.values[idx], 0); // eaten
    }

    // Check cached masks
    assert_eq!(next_board.opp_mask.popcount(), 4); // FIRST owned
    assert_eq!(next_board.my_mask.popcount(), 0);  // SECOND owned (none yet)
    assert!(!next_board.live_mask.get(0, 0));
}

#[test]
fn test_board_invalid_rect_move() {
    let board = create_sample_board();
    // 1+2 = 3 (not 10)
    let mv = (0, 0, 0, 1);
    assert!(!board.is_legal_action(mv));
}
