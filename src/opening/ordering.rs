use crate::board::Board;
use crate::dataloader::GameData;
use crate::eval::compute_move_features;
use crate::movegen::{fixed_rect_id, RectInfo};
use crate::mquality::MoveQualityTable;
use crate::types::*;

pub fn opening_move_bonus(
    board: &Board,
    mv: Move,
    rects: &[RectInfo],
    base_score: i32,
    use_mquality: bool,
    game_data: Option<&GameData>,
) -> i32 {
    let features = compute_move_features(board, mv);
    let board_live = board.live_mask.popcount() as i32;

    // Treat opening pressure earlier so the bonus is active before the board
    // becomes too cramped.
    if board_live <= 24 {
        return 0;
    }

    let phase = MoveQualityTable::phase_for_position(board.live_mask.popcount(), rects.len());
    let mut bonus = 0i32;

    bonus += features.corner * 40;
    bonus += features.edge * 15;

    if features.area >= 10 {
        bonus += features.area * 14;
    } else if features.area >= 7 {
        bonus += features.area * 8;
    } else if features.area >= 4 {
        bonus += features.area * 3;
    }

    bonus += features.recaptured * 20;
    bonus -= features.own * 15;
    bonus += features.fresh * 8;
    bonus += features.live * 5;

    if (mv.0 == 0 && mv.2 as usize == crate::types::ROWS - 1)
        || (mv.1 == 0 && mv.3 as usize == crate::types::COLS - 1)
    {
        bonus += 50;
    }

    let opp = crate::types::opponent(board.player);
    let mut opp_cells_nearby = 0i32;
    for r in (mv.0 as usize)..=(mv.2 as usize) {
        for c in (mv.1 as usize)..=(mv.3 as usize) {
            let idx = crate::types::cell_index(r, c);
            if board.owners[idx] == opp {
                opp_cells_nearby += 1;
            }
        }
    }
    bonus += opp_cells_nearby * 50;

    if use_mquality {
        if let Some(mquality_bonus) = game_data
            .and_then(|gd| gd.mquality.as_ref())
            .map(|mq| {
                let bucket = MoveQualityTable::score_bucket(base_score + bonus);
                let r_id = fixed_rect_id(mv.0, mv.1, mv.2, mv.3);
                mq.bonus(r_id as usize, phase, bucket) as i32
            })
        {
            bonus += mquality_bonus;
        }
    }

    bonus
}
