use mushroom_bot::board::Board;
use mushroom_bot::movegen::{generate_rectangles, RectInfo};
use mushroom_bot::types::*;

pub fn is_edge(mv: Move) -> bool {
    mv.0 == 0 || mv.2 as usize == ROWS - 1 || mv.1 == 0 || mv.3 as usize == COLS - 1
}

pub fn is_corner(mv: Move) -> bool {
    (mv.0 == 0 && mv.1 == 0)
        || (mv.0 == 0 && mv.3 as usize == COLS - 1)
        || (mv.2 as usize == ROWS - 1 && mv.1 == 0)
        || (mv.2 as usize == ROWS - 1 && mv.3 as usize == COLS - 1)
}

pub fn live_count(board: &Board) -> u32 {
    board.live_mask.popcount()
}

pub fn legal_rectangles(board: &Board) -> Vec<RectInfo> {
    generate_rectangles(&board.values)
}

pub fn reply_pressure(board: &Board, mv: Move) -> (i32, usize) {
    let next_state = board.apply_action(mv);
    let replies = generate_rectangles(&next_state.values);
    if replies.is_empty() {
        return (0, 0);
    }
    let best = replies
        .iter()
        .map(|reply| next_state.action_score(reply.to_move()).0)
        .max()
        .unwrap_or(0);
    (best, replies.len())
}

pub fn parse_u64(v: Option<&&str>) -> Option<u64> {
    v.and_then(|s| s.parse::<u64>().ok())
}

pub fn parse_move(parts: &[&str]) -> Option<Move> {
    if parts.len() < 4 {
        return None;
    }
    Some((
        parts[0].parse::<i8>().ok()?,
        parts[1].parse::<i8>().ok()?,
        parts[2].parse::<i8>().ok()?,
        parts[3].parse::<i8>().ok()?,
    ))
}

pub fn format_move(mv: Move) -> String {
    format!("{} {} {} {}", mv.0, mv.1, mv.2, mv.3)
}

pub fn stable_seed(mode: &str, rows: &[String], base_seed: u64) -> u64 {
    let mut seed = base_seed as u32;
    for ch in mode.chars() {
        seed = seed.wrapping_mul(131).wrapping_add(ch as u32);
    }
    for row in rows {
        for ch in row.chars() {
            seed = seed.wrapping_mul(131).wrapping_add(ch as u32);
        }
    }
    seed as u64
}
