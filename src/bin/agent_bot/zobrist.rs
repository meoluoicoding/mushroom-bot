use mushroom_bot::types::*;

static mut ZOBRIST_VALUE: [[[u64; 10]; COLS]; ROWS] = [[[0; 10]; COLS]; ROWS];
static mut ZOBRIST_OWNER: [[[u64; 3]; COLS]; ROWS] = [[[0; 3]; COLS]; ROWS];
static mut ZOBRIST_PLAYER: u64 = 0;
static mut ZOBRIST_INIT: bool = false;

fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

pub fn init_zobrist() {
    unsafe {
        if ZOBRIST_INIT { return; }
        let mut seed = 1234567890123456789u64;
        for r in 0..ROWS {
            for c in 0..COLS {
                for v in 0..10 {
                    ZOBRIST_VALUE[r][c][v] = xorshift64(&mut seed);
                }
            }
        }
        for r in 0..ROWS {
            for c in 0..COLS {
                for o in 0..3 {
                    ZOBRIST_OWNER[r][c][o] = xorshift64(&mut seed);
                }
            }
        }
        ZOBRIST_PLAYER = xorshift64(&mut seed);
        ZOBRIST_INIT = true;
    }
}

pub fn hash_board(values: &[u8; N_CELLS], owners: &[i8; N_CELLS], player: i8) -> u64 {
    init_zobrist();
    unsafe {
        let mut h: u64 = 0;
        for r in 0..ROWS {
            for c in 0..COLS {
                let idx = cell_index(r, c);
                let v = values[idx] as usize;
                if v > 0 && v <= 9 {
                    h ^= ZOBRIST_VALUE[r][c][v];
                }
                let o = owners[idx];
                let oi = if o == FIRST { 1 } else if o == SECOND { 2 } else { 0 };
                h ^= ZOBRIST_OWNER[r][c][oi];
            }
        }
        if player == FIRST {
            h ^= ZOBRIST_PLAYER;
        }
        h
    }
}
