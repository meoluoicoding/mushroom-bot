const TT_FLAG_EXACT: u8 = 1;
const TT_FLAG_LOWER: u8 = 2;
const TT_FLAG_UPPER: u8 = 3;

const TT_SIZE: usize = 1 << 22;

#[derive(Clone)]
struct TTEntry {
    key_sig: u32,
    depth: i16,
    value: i32,
    flag: u8,
    age: u8,
    packed_move: u32,
}

impl Default for TTEntry {
    fn default() -> Self {
        Self { key_sig: 0, depth: -1, value: 0, flag: 0, age: 0, packed_move: 0 }
    }
}

#[derive(Clone)]
struct TTBucket {
    slot0: TTEntry,
    slot1: TTEntry,
}

impl Default for TTBucket {
    fn default() -> Self {
        Self { slot0: TTEntry::default(), slot1: TTEntry::default() }
    }
}

pub struct TranspositionTable {
    table: Vec<TTBucket>,
    age: u8,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self { table: vec![TTBucket::default(); TT_SIZE], age: 1 }
    }

    pub fn increment_age(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    fn pack_move(r1: i8, c1: i8, r2: i8, c2: i8) -> u32 {
        ((r1 as u32 & 0xF) << 15)
            | ((c1 as u32 & 0x1F) << 10)
            | ((r2 as u32 & 0xF) << 6)
            | ((c2 as u32 & 0x1F) << 1)
    }

    const PACKED_PASS: u32 = 0xFFFFFFFF;

    fn unpack_move(packed: u32) -> (i8, i8, i8, i8) {
        if packed == Self::PACKED_PASS {
            return (-1, -1, -1, -1);
        }
        (
            ((packed >> 15) & 0xF) as i8,
            ((packed >> 10) & 0x1F) as i8,
            ((packed >> 6) & 0xF) as i8,
            ((packed >> 1) & 0x1F) as i8,
        )
    }

    pub fn store(&mut self, key: u64, depth: i16, value: i32, flag: u8, r1: i8, c1: i8, r2: i8, c2: i8) {
        let ksig = (key >> 32) as u32;
        let idx = key as usize & (TT_SIZE - 1);
        let bucket = &mut self.table[idx];
        let pm = Self::pack_move(r1, c1, r2, c2);

        bucket.slot0 = TTEntry { key_sig: ksig, depth, value, flag, age: self.age, packed_move: pm };

        let same_key = bucket.slot1.key_sig == ksig;
        let stale = bucket.slot1.age != self.age;
        let deeper = depth >= bucket.slot1.depth;
        if if same_key { deeper } else { stale || deeper } {
            bucket.slot1 = TTEntry { key_sig: ksig, depth, value, flag, age: self.age, packed_move: pm };
        }
    }

    pub fn probe(&self, key: u64, depth: i16, alpha: i32, beta: i32) -> (bool, i32, (i8, i8, i8, i8)) {
        let ksig = (key >> 32) as u32;
        let idx = key as usize & (TT_SIZE - 1);
        let bucket = &self.table[idx];

        let mut best: Option<&TTEntry> = None;
        if bucket.slot0.key_sig == ksig && bucket.slot0.depth >= depth {
            best = Some(&bucket.slot0);
        }
        if bucket.slot1.key_sig == ksig && bucket.slot1.depth >= depth {
            if best.is_none() || bucket.slot1.depth > best.unwrap().depth {
                best = Some(&bucket.slot1);
            }
        }

        match best {
            None => (false, 0, (-1, -1, -1, -1)),
            Some(e) => {
                let bm = Self::unpack_move(e.packed_move);
                if e.flag == TT_FLAG_EXACT { return (true, e.value, bm); }
                if e.flag == TT_FLAG_LOWER && e.value >= beta { return (true, e.value, bm); }
                if e.flag == TT_FLAG_UPPER && e.value <= alpha { return (true, e.value, bm); }
                (false, 0, bm)
            }
        }
    }
}
