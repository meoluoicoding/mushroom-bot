use crate::board::Board;
use crate::dataloader::GameData;
use crate::eval::compute_move_features;
use crate::movegen::RectInfo;
use crate::types::*;

pub fn midgame_move_bonus(
    board: &Board,
    mv: Move,
    _rects: &[RectInfo],
    _game_data: Option<&GameData>,
) -> i32 {
    let features = compute_move_features(board, mv);
    let board_live = board.live_mask.popcount() as i32;

    if board_live < 13 || board_live > 32 {
        return 0;
    }

    let mut bonus = 0i32;

    bonus += features.recaptured * 10;
    bonus += features.fresh * 3;
    bonus += features.edge * 5;

    if features.area >= 6 {
        bonus += features.area * 3;
    }

    if board_live <= 18 {
        bonus += features.corner * 15;
        bonus += features.recaptured * 8;
    }

    bonus
}
