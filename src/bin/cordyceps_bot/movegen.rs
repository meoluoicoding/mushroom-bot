use mushroom_bot::board::Board;
use mushroom_bot::movegen::generate_rectangles;

pub type CordycepsBoard = Board;

pub fn generate_moves(board: &Board) -> Vec<(i8, i8, i8, i8)> {
    let rects = generate_rectangles(&board.values);
    rects.iter().map(|r| (r.r1, r.c1, r.r2, r.c2)).collect()
}

pub fn live_mushrooms(board: &Board) -> i32 {
    board.values.iter().filter(|&&v| v > 0).count() as i32
}
