use crate::board::Board;
use crate::types::*;

pub fn stealth_bonus(board: &Board, mv: Move, child: &Board) -> f32 {
    if mv == PASS {
        return 0.0;
    }

    let live = board.live_mask.popcount();
    if live < 24 {
        return 0.0;
    }

    let (r1, c1, r2, c2) = mv;

    let child_opp_live = child.live_mask.popcount() as f32;

    let area = ((r2 - r1 + 1) * (c2 - c1 + 1)) as f32;
    let density_penalty = if area >= 12.0 { 8.0 } else if area >= 8.0 { 3.0 } else { 0.0 };

    let center_x = (crate::types::ROWS / 2) as i8;
    let center_y = (crate::types::COLS / 2) as i8;
    let dx = ((r1 + r2) / 2 - center_x).abs() as f32;
    let dy = ((c1 + c2) / 2 - center_y).abs() as f32;
    let center_dist = (dx * dx + dy * dy).sqrt();
    let center_bonus = if center_dist < 3.0 { 10.0 } else if center_dist < 6.0 { 5.0 } else { 0.0 };

    let fragmentation_bonus = if child_opp_live > live as f32 / 2.0 { 7.0 } else { 0.0 };

    -density_penalty + center_bonus + fragmentation_bonus
}
