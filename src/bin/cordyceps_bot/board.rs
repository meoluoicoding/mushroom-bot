use mushroom_bot::board::Board;
use mushroom_bot::types::*;

pub type CordycepsBoard = Board;

pub fn legal_moves(board: &Board) -> Vec<(i8, i8, i8, i8)> {
    use mushroom_bot::movegen::generate_rectangles;
    generate_rectangles(&board.values).iter().map(|r| (r.r1, r.c1, r.r2, r.c2)).collect()
}

pub fn is_terminal(board: &Board) -> bool {
    board.passes >= 2
}

pub fn terminal_score(board: &Board, player: i8) -> f32 {
    let my = board.my_mask.popcount() as i32;
    let opp = board.opp_mask.popcount() as i32;
    let margin = if player == FIRST { my - opp } else { opp - my };
    if margin > 0 { 500000.0 + margin as f32 * 100.0 }
    else if margin < 0 { -500000.0 + margin as f32 * 100.0 }
    else { 0.0 }
}
