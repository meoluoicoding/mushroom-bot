pub const ROWS: usize = 10;
pub const COLS: usize = 17;
pub const N_CELLS: usize = ROWS * COLS;
pub const FIRST: i8 = 1;
pub const SECOND: i8 = -1;
pub const PASS: (i8, i8, i8, i8) = (-1, -1, -1, -1);
pub const N_RECTS: usize = ROWS * (ROWS + 1) / 2 * COLS * (COLS + 1) / 2;

pub type Move = (i8, i8, i8, i8);

pub use crate::bitboard::Bitboard;

pub fn cell_index(r: usize, c: usize) -> usize {
    r * COLS + c
}

pub fn opponent(p: i8) -> i8 {
    -p
}
