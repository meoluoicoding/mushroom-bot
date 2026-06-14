use crate::board::AgentBoard;
use mushroom_bot::dataloader::EvalWeights;
use mushroom_bot::movegen::{generate_rectangles, RectInfo};
use mushroom_bot::types::*;

pub static mut EVAL_CALLS: u64 = 0;

pub fn evaluate(board: &AgentBoard, w: &EvalWeights) -> i32 {
    unsafe { EVAL_CALLS += 1; }
    let player = board.player;
    let opp = opponent(player);

    let mut own_cells = 0i32;
    let mut opp_cells = 0i32;
    for &o in &board.owners {
        if o == player { own_cells += 1; }
        if o == opp { opp_cells += 1; }
    }
    let territory = own_cells - opp_cells;

    let mut connectivity = 0i32;
    for r in 0..ROWS {
        for c in 0..COLS {
            let idx = cell_index(r, c);
            if board.owners[idx] != player { continue; }
            if c + 1 < COLS && board.owners[cell_index(r, c + 1)] == player { connectivity += 1; }
            if r + 1 < ROWS && board.owners[cell_index(r + 1, c)] == player { connectivity += 1; }
        }
    }

    let mut corners = 0i32;
    let mut edges = 0i32;
    let corner_pos: [(usize, usize); 4] = [(0, 0), (0, COLS - 1), (ROWS - 1, 0), (ROWS - 1, COLS - 1)];
    for (cr, cc) in corner_pos {
        let o = board.owners[cell_index(cr, cc)];
        if o == player { corners += 1; } else if o == opp { corners -= 1; }
    }
    for c in 0..COLS {
        if board.owners[cell_index(0, c)] == player { edges += 1; }
        else if board.owners[cell_index(0, c)] == opp { edges -= 1; }
        if board.owners[cell_index(ROWS - 1, c)] == player { edges += 1; }
        else if board.owners[cell_index(ROWS - 1, c)] == opp { edges -= 1; }
    }
    for r in 1..ROWS - 1 {
        if board.owners[cell_index(r, 0)] == player { edges += 1; }
        else if board.owners[cell_index(r, 0)] == opp { edges -= 1; }
        if board.owners[cell_index(r, COLS - 1)] == player { edges += 1; }
        else if board.owners[cell_index(r, COLS - 1)] == opp { edges -= 1; }
    }

    let mut recapture_swing = 0i32;
    let mut vulnerability = 0i32;
    for r in 0..ROWS {
        for c in 0..COLS {
            let idx = cell_index(r, c);
            let mut adjacent_to_live = false;
            if r > 0 && board.values[cell_index(r - 1, c)] > 0 { adjacent_to_live = true; }
            if r < ROWS - 1 && board.values[cell_index(r + 1, c)] > 0 { adjacent_to_live = true; }
            if c > 0 && board.values[cell_index(r, c - 1)] > 0 { adjacent_to_live = true; }
            if c < COLS - 1 && board.values[cell_index(r, c + 1)] > 0 { adjacent_to_live = true; }
            if adjacent_to_live {
                if board.owners[idx] == opp { recapture_swing += 1; }
                else if board.owners[idx] == player { vulnerability += 1; }
            }
        }
    }

    let t = w.territory as i32;
    let c = w.connectivity as i32;
    let cb = w.corner_bonus as i32;
    let eb = w.edge_bonus as i32;
    let sp = w.steal_potential as i32;
    let vp = w.vulnerability as i32;

    territory * t + connectivity * c + corners * cb + edges * eb + recapture_swing * sp - vulnerability * vp
}

pub fn score_move(board: &AgentBoard, r1: i8, c1: i8, r2: i8, c2: i8, w: &EvalWeights) -> i32 {
    let mut copy = board.clone();
    copy.apply_move(r1, c1, r2, c2);
    evaluate(&copy, w)
}

pub fn legal_moves_for(board: &AgentBoard) -> Vec<(i8, i8, i8, i8)> {
    let rects = generate_rectangles(&board.values);
    rects.iter().map(|r: &RectInfo| (r.r1, r.c1, r.r2, r.c2)).collect()
}

pub fn legal_moves_prioritized(board: &AgentBoard) -> Vec<(i32, i8, i8, i8, i8)> {
    let rects = generate_rectangles(&board.values);
    let mut moves: Vec<(i32, i8, i8, i8, i8)> = Vec::with_capacity(rects.len());
    for r in &rects {
        let mut steal = 0i32;
        let mut recaptured = 0i32;
        let mut fresh = 0i32;
        let mut own = 0i32;
        let area = (r.r2 - r.r1 + 1) as i32 * (r.c2 - r.c1 + 1) as i32;
        for rr in r.r1..=r.r2 {
            for cc in r.c1..=r.c2 {
                let idx = cell_index(rr as usize, cc as usize);
                match board.owners[idx] {
                    o if o == opponent(board.player) => {
                        steal += 1;
                        recaptured += 1;
                    }
                    0 => fresh += 1,
                    _ => own += 1,
                }
            }
        }
        let height = r.r2 - r.r1 + 1;
        let width = r.c2 - r.c1 + 1;
        let portrait_bonus = if height > width { 500 } else { 0 };
        let small_bonus = if area <= 4 { 300 } else { 0 };
        let recapture_bonus = if recaptured > 0 {
            2_500 + recaptured * 1_600 + fresh * 120
        } else {
            0
        };
        let efficiency_bonus = if area <= 6 { (6 - area) * 140 } else { 0 };
        let penalty = own * 60;
        let priority = steal * 900 + recapture_bonus + efficiency_bonus + area * 12 + portrait_bonus + small_bonus - penalty;
        moves.push((priority, r.r1, r.c1, r.r2, r.c2));
    }
    moves.sort_by_key(|m| -m.0);
    moves
}
