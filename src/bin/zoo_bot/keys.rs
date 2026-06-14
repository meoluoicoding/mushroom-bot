use mushroom_bot::board::Board;
use mushroom_bot::types::*;

use crate::utils::{is_corner, is_edge, reply_pressure};

pub fn move_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn area_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        area,
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn recapture_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        recaptured,
        score_delta,
        fresh,
        live,
        net_area,
        area,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn fresh_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        fresh,
        score_delta,
        recaptured,
        live,
        net_area,
        area,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn edge_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        is_corner(mv) as i32,
        is_edge(mv) as i32,
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
    )
}

pub fn corner_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        is_corner(mv) as i32,
        is_edge(mv) as i32,
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
    )
}

pub fn reply_aware_key(board: &Board, mv: Move, reply_weight: f64) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let immediate = board.action_score(mv);
    let (reply_score, reply_count) = reply_pressure(board, mv);
    let mobility_penalty = (reply_count.min(16) as f64) * 0.03;
    let score = ((immediate.0 as f64) - reply_weight * reply_score as f64 - mobility_penalty).round() as i32;
    (
        score,
        immediate.1,
        immediate.2,
        immediate.3,
        immediate.4,
        immediate.5,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn defensive_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let opponent = mushroom_bot::types::opponent(board.player);
    let lead = board.score(board.player) - board.score(opponent);
    let immediate = board.action_score(mv);
    let (reply_score, reply_count) = reply_pressure(board, mv);
    let (weight, area_penalty) = if lead > 0 {
        (0.90 + (lead.min(12) as f64) * 0.06, (immediate.5.min(30) as f64) * 0.10)
    } else {
        (0.70, (immediate.5.min(30) as f64) * 0.04)
    };
    let mobility_penalty = (reply_count.min(18) as f64) * 0.03;
    let score = ((immediate.0 as f64) - weight * reply_score as f64 - mobility_penalty - area_penalty).round() as i32;
    (
        score,
        immediate.1,
        immediate.2,
        immediate.3,
        immediate.4,
        immediate.5,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn defensive_when_losing_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let opponent = mushroom_bot::types::opponent(board.player);
    let lead = board.score(board.player) - board.score(opponent);
    let immediate = board.action_score(mv);
    let (reply_score, reply_count) = reply_pressure(board, mv);
    let (weight, area_penalty) = if lead > 0 {
        (0.78, (immediate.5.min(30) as f64) * 0.03)
    } else {
        (0.98 + (lead.abs().min(12) as f64) * 0.04, (immediate.5.min(30) as f64) * 0.12)
    };
    let mobility_penalty = (reply_count.min(18) as f64) * 0.04;
    let score = ((immediate.0 as f64) - weight * reply_score as f64 - mobility_penalty - area_penalty).round() as i32;
    (
        score,
        immediate.1,
        immediate.2,
        immediate.3,
        immediate.4,
        immediate.5,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn net_area_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        net_area,
        score_delta,
        recaptured,
        fresh,
        live,
        area,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn live_cell_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    (
        live,
        score_delta,
        recaptured,
        fresh,
        net_area,
        area,
        -(mv.0 as i32),
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}

pub fn position_key(board: &Board, mv: Move) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let (score_delta, recaptured, fresh, live, net_area, area) = board.action_score(mv);
    let center_row = (mv.0 as i32 + mv.2 as i32 - 9).abs();
    let center_col = (mv.1 as i32 + mv.3 as i32 - 16).abs();
    (
        -(center_row + center_col),
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
        -(mv.1 as i32 + mv.2 as i32 + mv.3 as i32),
    )
}
