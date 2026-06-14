use mushroom_bot::timeman::{TimeManager, SearchPhase};

#[test]
fn test_timeman_budget_sanity() {
    let mut tm = TimeManager::new();

    // Case 1: Plentiful time (10 seconds)
    tm.update(10_000, 10_000);
    let budget_full = tm.search_budget_ms(20, 15); // Midgame phase
    assert!(budget_full > 0);
    assert!(budget_full <= tm.my_time_left_ms);

    // Case 2: Low time (800ms)
    tm.update(800, 1000);
    let budget_low = tm.search_budget_ms(20, 15);
    assert!(budget_low > 0);
    assert!(budget_low <= tm.my_time_left_ms);
    assert!(budget_low < budget_full, "Budget must decrease when time is low");

    // Case 3: Emergency time (150ms)
    tm.update(150, 1000);
    let budget_emergency = tm.search_budget_ms(20, 15);
    assert!(budget_emergency > 0);
    assert!(budget_emergency <= tm.my_time_left_ms);
    assert!(budget_emergency < budget_low);

    // Case 4: Endgame phase (low live count)
    tm.update(5000, 5000);
    let budget_endgame = tm.search_budget_ms(10, 15); // live_count <= 12 is endgame
    assert!(budget_endgame > 0);
    assert!(budget_endgame <= tm.my_time_left_ms);
    
    // With 5000ms left, usable = 5000 - reserve = 4800.
    // 60% of usable is 2880ms. Let's make sure it's close to that.
    assert_eq!(budget_endgame, (5000 - tm.reserve_ms) * 60 / 100);

    // Case 5: Endgame phase (low legal moves)
    let budget_endgame_moves = tm.search_budget_ms(20, 5); // legal_moves <= 10 is endgame
    assert_eq!(budget_endgame_moves, (5000 - tm.reserve_ms) * 60 / 100);
}

#[test]
fn test_timeman_phases() {
    let mut tm = TimeManager::new();

    // Plentiful time, many mushrooms
    tm.update(5000, 5000);
    assert!(matches!(tm.phase(20, 15), SearchPhase::MidgameFull));

    // Midgame Conserve
    tm.update(1500, 1500);
    assert!(matches!(tm.phase(20, 15), SearchPhase::MidgameConserve));

    // Emergency
    tm.update(400, 400);
    assert!(matches!(tm.phase(20, 15), SearchPhase::Emergency));

    // Endgame (low live count)
    tm.update(5000, 5000);
    assert!(matches!(tm.phase(10, 5), SearchPhase::Endgame));
}
