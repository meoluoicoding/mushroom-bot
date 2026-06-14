use crate::board::Board;
use crate::dataloader::GameData;
use crate::eval::compute_move_features;
use crate::movegen::RectInfo;
use crate::types::*;

pub fn endgame_move_bonus(
    board: &Board,
    mv: Move,
    _rects: &[RectInfo],
    _game_data: Option<&GameData>,
) -> i32 {
    let features = compute_move_features(board, mv);
    let board_live = board.live_mask.popcount() as i32;

    if board_live > 12 {
        return 0;
    }

    let mut bonus = 0i32;

    bonus += features.recaptured * 30;
    bonus += features.fresh * 5;
    bonus += features.corner * 20;
    bonus += features.edge * 12;

    if features.area >= 4 {
        bonus += features.area * 4;
    }

    let opponent_score = board.opp_mask.popcount() as i32;
    if opponent_score > board.my_mask.popcount() as i32 && features.recaptured > 0 {
        bonus += 40;
    }

    bonus
}
