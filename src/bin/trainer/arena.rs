use mushroom_bot::board::Board;
use mushroom_bot::movegen::generate_rectangles;
use mushroom_bot::types::*;
use std::time::Instant;

use crate::bots::BotFn;

#[derive(Clone)]
pub struct MoveLog {
    pub game_id: u32,
    pub ply: u32,
    pub bot_a: u32,
    pub bot_b: u32,
    pub mover_is_a: bool,
    pub rect_id: u16,
    pub score_delta: i32,
    pub recaptured: i32,
    pub fresh: i32,
    pub live: i32,
    pub net_area: i32,
    pub area: i32,
    pub live_count: u32,
    pub num_moves: usize,
    pub outcome: f32,
    pub margin: i32,
    pub elapsed_ms: f64,
}

fn random_board(seed: u64) -> Board {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut values = [0u8; N_CELLS];
    for v in &mut values {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *v = (state % 9 + 1) as u8;
    }
    Board::from_parts(values, [0i8; N_CELLS], FIRST, 0)
}

pub fn play_match(
    bot_a: (&'static str, BotFn),
    bot_b: (&'static str, BotFn),
    seed: u64,
    first_goes_first: bool,
    budget_ms: u64,
    a_idx: u32, b_idx: u32, _game_n: u32,
) -> (i32, Vec<MoveLog>) {
    let mut board = random_board(seed);
    let mut logs = Vec::new();
    let mut ply = 0u32;
    let mut turn_a = first_goes_first;
    let start = Instant::now();

    loop {
        if board.is_terminal() { break; }
        let rects = generate_rectangles(&board.values);
        if rects.is_empty() {
            board = board.apply_action(PASS);
            turn_a = !turn_a;
            ply += 1;
            continue;
        }

        let active_bot = if turn_a { bot_a.1 } else { bot_b.1 };
        let move_start = Instant::now();
        let mv = active_bot(&board, budget_ms);
        let move_elapsed = move_start.elapsed().as_secs_f64() * 1000.0;

        if !board.is_legal_action(mv) { break; }

        let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
        let live_count = board.live_mask.popcount();
        let num_moves = rects.len();
        let rect_id = if mv == PASS { u16::MAX } else { 0u16 };

        logs.push(MoveLog {
            game_id: seed as u32, ply, bot_a: a_idx, bot_b: b_idx,
            mover_is_a: turn_a, rect_id, score_delta, recaptured, fresh, live,
            net_area, area, live_count, num_moves,
            outcome: 0.0, margin: 0, elapsed_ms: move_elapsed,
        });

        board = board.apply_action(mv);
        turn_a = !turn_a;
        ply += 1;
    }

    let margin = board.score(FIRST) - board.score(SECOND);
    for log in &mut logs {
        log.margin = margin;
        log.outcome = if turn_a { -outcome(margin) } else { outcome(margin) };
    }

    let total_elapsed = start.elapsed().as_secs_f64() * 1000.0;
    for log in &mut logs { log.elapsed_ms = total_elapsed; }

    (margin, logs)
}

fn outcome(margin: i32) -> f32 {
    if margin > 0 { 1.0 } else if margin < 0 { 0.0 } else { 0.5 }
}
