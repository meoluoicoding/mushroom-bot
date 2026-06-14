use crate::types::*;

#[derive(Clone, Copy, Debug)]
pub struct RectInfo {
    pub id: u16,
    pub r1: i8,
    pub c1: i8,
    pub r2: i8,
    pub c2: i8,
    pub area: u8,
    pub cell_mask: Bitboard,
}

impl RectInfo {
    pub fn to_move(&self) -> Move {
        (self.r1, self.c1, self.r2, self.c2)
    }
}

#[inline]
pub fn fixed_rect_id(r1: i8, c1: i8, r2: i8, c2: i8) -> u16 {
    let r1 = r1 as usize;
    let r2 = r2 as usize;
    let c1 = c1 as usize;
    let c2 = c2 as usize;

    let r_term = if r1 > 0 { r1 * (r1 - 1) / 2 } else { 0 };
    let r_index = r1 * ROWS - r_term + (r2 - r1);

    let c_term = if c1 > 0 { c1 * (c1 - 1) / 2 } else { 0 };
    let c_index = c1 * COLS - c_term + (c2 - c1);

    let cols_pairs = COLS * (COLS + 1) / 2;

    (r_index * cols_pairs + c_index) as u16
}

#[inline]
pub fn rect_mask(r1: usize, r2: usize, c1: usize, c2: usize) -> Bitboard {
    let mut bb = Bitboard::empty();
    let col_bits: u64 = if c2 - c1 + 1 >= 64 {
        u64::MAX
    } else {
        ((1u64 << (c2 - c1 + 1)) - 1) << c1
    };
    for r in r1..=r2 {
        let base_bit = r * COLS;
        let word = base_bit / 64;
        let shift = base_bit % 64;
        bb.0[word] |= col_bits << shift;
        if shift + COLS > 64 && word + 1 < 3 {
            bb.0[word + 1] |= col_bits >> (64 - shift);
        }
    }
    bb
}


pub fn generate_rectangles(values: &[u8; N_CELLS]) -> Vec<RectInfo> {
    let mut vp = [[0i32; COLS + 1]; ROWS + 1];
    let mut lp = [[0i32; COLS + 1]; ROWS + 1];
    let mut lpc = [[0i32; ROWS + 1]; COLS + 1];
    prefix_sum(values, &mut vp);
    live_prefix(values, &mut lp);
    live_prefix_col(values, &mut lpc);

    let mut rects: Vec<RectInfo> = Vec::with_capacity(256);

    for r1 in 0..ROWS {
        for r2 in r1..ROWS {
            let mut col_sums = [0i32; COLS];
            for c in 0..COLS {
                col_sums[c] = vp[r2 + 1][c + 1] - vp[r1][c + 1] - vp[r2 + 1][c] + vp[r1][c];
            }

            let mut c1 = 0;
            while c1 < COLS {
                let mut total = 0i32;
                let mut c2 = c1;
                while c2 < COLS {
                    total += col_sums[c2];
                    if total > 10 { break; }
                    if total == 10 {
                        let top = rect_sum_row(&lp, r1, c1, r1, c2) > 0;
                        let bottom = rect_sum_row(&lp, r2, c1, r2, c2) > 0;
                        let left = rect_sum_col(&lpc, c1, r1, c1, r2) > 0;
                        let right = rect_sum_col(&lpc, c2, r1, c2, r2) > 0;
                        if top && bottom && left && right {
                            let cm = rect_mask(r1, r2, c1, c2);
                            rects.push(RectInfo {
                                id: fixed_rect_id(r1 as i8, c1 as i8, r2 as i8, c2 as i8),
                                r1: r1 as i8, c1: c1 as i8,
                                r2: r2 as i8, c2: c2 as i8,
                                area: ((r2 - r1 + 1) * (c2 - c1 + 1)) as u8,
                                cell_mask: cm,
                            });
                        }
                    }
                    c2 += 1;
                }
                c1 += 1;
            }
        }
    }
    rects
}

pub fn is_valid_rectangle(values: &[u8; N_CELLS], action: Move) -> bool {
    let (r1, c1, r2, c2) = action;
    if r1 < 0 || c1 < 0 || r2 >= ROWS as i8 || c2 >= COLS as i8 || r1 > r2 || c1 > c2 {
        return false;
    }
    let (r1, c1, r2, c2) = (r1 as usize, c1 as usize, r2 as usize, c2 as usize);
    let mut total = 0i32;
    let (mut top, mut bottom, mut left, mut right) = (false, false, false, false);
    for r in r1..=r2 {
        for c in c1..=c2 {
            let v = values[cell_index(r, c)] as i32;
            if v <= 0 { continue; }
            total += v;
            if total > 10 { return false; }
            if r == r1 { top = true; }
            if r == r2 { bottom = true; }
            if c == c1 { left = true; }
            if c == c2 { right = true; }
        }
    }
    total == 10 && top && bottom && left && right
}

fn prefix_sum(values: &[u8; N_CELLS], p: &mut [[i32; COLS + 1]; ROWS + 1]) {
    for r in 0..ROWS {
        let mut rt = 0i32;
        for c in 0..COLS {
            rt += values[cell_index(r, c)] as i32;
            p[r + 1][c + 1] = p[r][c + 1] + rt;
        }
    }
}

fn live_prefix(values: &[u8; N_CELLS], p: &mut [[i32; COLS + 1]; ROWS + 1]) {
    for r in 0..ROWS {
        let mut rt = 0i32;
        for c in 0..COLS {
            rt += (values[cell_index(r, c)] > 0) as i32;
            p[r + 1][c + 1] = p[r][c + 1] + rt;
        }
    }
}

fn live_prefix_col(values: &[u8; N_CELLS], p: &mut [[i32; ROWS + 1]; COLS + 1]) {
    for c in 0..COLS {
        let mut ct = 0i32;
        for r in 0..ROWS {
            ct += (values[cell_index(r, c)] > 0) as i32;
            p[c + 1][r + 1] = p[c][r + 1] + ct;
        }
    }
}

#[inline]
fn rect_sum_row(p: &[[i32; COLS + 1]; ROWS + 1], r1: usize, c1: usize, r2: usize, c2: usize) -> i32 {
    p[r2 + 1][c2 + 1] - p[r1][c2 + 1] - p[r2 + 1][c1] + p[r1][c1]
}

#[inline]
fn rect_sum_col(p: &[[i32; ROWS + 1]; COLS + 1], r1: usize, c1: usize, r2: usize, c2: usize) -> i32 {
    p[r2 + 1][c2 + 1] - p[r1][c2 + 1] - p[r2 + 1][c1] + p[r1][c1]
}
