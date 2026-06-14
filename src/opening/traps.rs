use crate::board::Board;
use crate::dataloader::{EvalWeights, GameData};
use crate::types::*;
use crate::movegen::generate_rectangles;

pub fn trapbook_bonus(
    board: &Board,
    mv: Move,
    child: &Board,
    _weights: &EvalWeights,
    _game_data: Option<&GameData>,
) -> f32 {
    if mv == PASS {
        return 0.0;
    }

    let child_rects = generate_rectangles(&child.values);
    let opp_options = child_rects.len();

    if opp_options == 0 {
        return 50.0;
    }
    if opp_options <= 2 {
        return 25.0;
    }
    if opp_options <= 5 {
        return 10.0;
    }

    let before_live = board.live_mask.popcount();
    let after_live = child.live_mask.popcount();
    let live_drop = before_live.saturating_sub(after_live) as f32;
    if live_drop >= 20.0 {
        return 15.0;
    }
    if live_drop >= 12.0 {
        return 8.0;
    }

    0.0
}

pub fn tactical_squeeze_bonus(
    child: &Board,
    _weights: &EvalWeights,
    _game_data: Option<&GameData>,
) -> f32 {
    let child_rects = generate_rectangles(&child.values);

    let opp_best_reply = child_rects.iter().map(|r| {
        let reply_mv = r.to_move();
        let (_sd, recaptured, fresh, _live, _own, area) = child.action_score(reply_mv);
        (recaptured * 30 + fresh * 10 + area * 10) as f32
    }).fold(0.0f32, f32::max);

    if opp_best_reply < 60.0 {
        return 30.0;
    }
    if opp_best_reply < 120.0 {
        return 15.0;
    }
    if opp_best_reply < 200.0 {
        return 5.0;
    }

    0.0
}
