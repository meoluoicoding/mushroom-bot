use crate::types::COLS;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bitboard(pub [u64; 3]);

impl Bitboard {
    #[inline]
    pub const fn empty() -> Self {
        Bitboard([0; 3])
    }

    #[inline]
    pub fn set(&mut self, r: usize, c: usize) {
        let idx = r * COLS + c;
        self.0[idx / 64] |= 1u64 << (idx % 64);
    }

    #[inline]
    pub fn clear(&mut self, r: usize, c: usize) {
        let idx = r * COLS + c;
        self.0[idx / 64] &= !(1u64 << (idx % 64));
    }

    #[inline]
    pub fn get(&self, r: usize, c: usize) -> bool {
        let idx = r * COLS + c;
        (self.0[idx / 64] >> (idx % 64)) & 1 != 0
    }

    #[inline]
    pub fn popcount(&self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones() + self.0[2].count_ones()
    }

    #[inline]
    pub fn and(&self, other: &Bitboard) -> Bitboard {
        Bitboard([
            self.0[0] & other.0[0],
            self.0[1] & other.0[1],
            self.0[2] & other.0[2],
        ])
    }

    #[inline]
    pub fn or(&self, other: &Bitboard) -> Bitboard {
        Bitboard([
            self.0[0] | other.0[0],
            self.0[1] | other.0[1],
            self.0[2] | other.0[2],
        ])
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0
    }
}
