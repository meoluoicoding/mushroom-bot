use crate::types::*;

const TT_SIZE: usize = 1 << 22; // 4M entries
const TT_MASK: usize = TT_SIZE - 1;

// Zobrist keys for proper board hashing
// Generated deterministically from a seed
static ZOBRIST: std::sync::LazyLock<ZobristKeys> = std::sync::LazyLock::new(ZobristKeys::new);

pub struct ZobristKeys {
    // Keys for cell values (0-10) at each position
    pub values: [[u64; 11]; N_CELLS],
    // Keys for cell ownership (-1, 0, 1) at each position
    pub owners: [[u64; 3]; N_CELLS],
    // Key for current player
    pub player: [u64; 2],
    // Key for pass count
    pub passes: [u64; 3],
}

impl ZobristKeys {
    fn new() -> Self {
        let mut rng = Rng::new(0x3141592653589793);

        let mut values = [[0u64; 11]; N_CELLS];
        let mut owners = [[0u64; 3]; N_CELLS];

        for i in 0..N_CELLS {
            for v in 0..11 {
                values[i][v] = rng.next();
            }
            for o in 0..3 {
                owners[i][o] = rng.next();
            }
        }

        let player = [rng.next(), rng.next()];
        let passes = [rng.next(), rng.next(), rng.next()];

        Self {
            values,
            owners,
            player,
            passes,
        }
    }
}

/// Simple deterministic RNG for Zobrist key generation
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub depth: u8,
    pub value: f32,
    pub flag: u8,
    pub best_move: Option<Move>,
}

pub const EXACT: u8 = 0;
pub const LOWER: u8 = 1;
pub const UPPER: u8 = 2;

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            entries: vec![None; TT_SIZE],
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(None);
    }

    pub fn probe(
        &self,
        key: u64,
        depth: u8,
        alpha: f32,
        beta: f32,
    ) -> Option<(f32, u8, Option<Move>)> {
        let idx = (key as usize) & TT_MASK;
        let entry = self.entries[idx]?;
        if entry.key != key || entry.depth < depth {
            return None;
        }
        match entry.flag {
            EXACT => Some((entry.value, entry.flag, entry.best_move)),
            LOWER => {
                if entry.value >= beta {
                    Some((entry.value, entry.flag, entry.best_move))
                } else {
                    None
                }
            }
            UPPER => {
                if entry.value <= alpha {
                    Some((entry.value, entry.flag, entry.best_move))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

        pub fn store(&mut self, key: u64, depth: u8, value: f32, flag: u8, best_move: Option<Move>) {
            let idx = (key as usize) & TT_MASK;
            // Keep same-key entries first; otherwise prefer deeper search results.
            let should_replace = match self.entries[idx] {
                None => true,
                Some(e) => {
                    if e.key == key {
                        true
                    } else {
                        e.depth <= depth
                    }
                }
            };
            if should_replace {
                self.entries[idx] = Some(TTEntry {
                key,
                depth,
                value,
                flag,
                best_move,
            });
        }
    }

    pub fn get_best_move(&self, key: u64) -> Option<Move> {
        let idx = (key as usize) & TT_MASK;
        self.entries[idx].and_then(|e| if e.key == key { e.best_move } else { None })
    }

    pub fn get_entry(&self, key: u64) -> Option<TTEntry> {
        let idx = (key as usize) & TT_MASK;
        let entry = self.entries[idx]?;
        if entry.key == key {
            Some(entry)
        } else {
            None
        }
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute Zobrist hash for a board position
pub fn hash_board(values: &[u8; N_CELLS], owners: &[i8; N_CELLS], player: i8, passes: u8) -> u64 {
    let z = &*ZOBRIST;
    let mut h: u64 = 0;

    // Hash cell values and owners
    for i in 0..N_CELLS {
        h ^= z.values[i][values[i] as usize];
        let owner_idx = (owners[i] + 1) as usize; // -1 -> 0, 0 -> 1, 1 -> 2
        h ^= z.owners[i][owner_idx];
    }

    // Hash player to move
    let player_idx = if player == FIRST { 0 } else { 1 };
    h ^= z.player[player_idx];

    // Hash pass count
    h ^= z.passes[passes as usize % 3];

    h
}

/// Incrementally update hash after a move
pub fn hash_update(
    hash: u64,
    values: &[u8; N_CELLS],
    owners: &[i8; N_CELLS],
    mv: Move,
    player: i8,
    old_passes: u8,
    new_passes: u8,
) -> u64 {
    let z = &*ZOBRIST;
    let mut h = hash;

    // Toggle player
    let old_player_idx = if player == FIRST { 0 } else { 1 };
    let new_player_idx = 1 - old_player_idx;
    h ^= z.player[old_player_idx];
    h ^= z.player[new_player_idx];

    // Toggle passes
    h ^= z.passes[old_passes as usize % 3];
    h ^= z.passes[new_passes as usize % 3];

    if mv != PASS {
        let (r1, c1, r2, c2) = mv;
        let owner_idx = (player + 1) as usize; // Convert player to owner index

        for r in r1 as usize..=r2 as usize {
            for c in c1 as usize..=c2 as usize {
                let idx = r * COLS + c;

                // Remove old value
                h ^= z.values[idx][values[idx] as usize];
                // Add new value (0)
                h ^= z.values[idx][0];

                // Remove old owner
                let old_owner_idx = (owners[idx] + 1) as usize;
                h ^= z.owners[idx][old_owner_idx];
                // Add new owner
                h ^= z.owners[idx][owner_idx];
            }
        }
    }

    h
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zobrist_different_positions() {
        let values1 = [1u8; N_CELLS];
        let owners1 = [0i8; N_CELLS];

        let mut values2 = [1u8; N_CELLS];
        values2[0] = 2;
        let owners2 = [0i8; N_CELLS];

        let h1 = hash_board(&values1, &owners1, FIRST, 0);
        let h2 = hash_board(&values2, &owners2, FIRST, 0);

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_zobrist_different_players() {
        let values = [1u8; N_CELLS];
        let owners = [0i8; N_CELLS];

        let h1 = hash_board(&values, &owners, FIRST, 0);
        let h2 = hash_board(&values, &owners, SECOND, 0);

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_tt_basic() {
        let mut tt = TranspositionTable::new();
        let key = 12345u64;

        tt.store(key, 5, 100.0, EXACT, Some((0, 0, 1, 1)));

        let result = tt.probe(key, 5, f32::NEG_INFINITY, f32::INFINITY);
        assert!(result.is_some());
        let (val, flag, mv) = result.unwrap();
        assert_eq!(val, 100.0);
        assert_eq!(flag, EXACT);
        assert_eq!(mv, Some((0, 0, 1, 1)));
    }

    #[test]
    fn test_tt_depth_check() {
        let mut tt = TranspositionTable::new();
        let key = 12345u64;

        tt.store(key, 3, 100.0, EXACT, None);

        // Should not return for deeper search
        let result = tt.probe(key, 5, f32::NEG_INFINITY, f32::INFINITY);
        assert!(result.is_none());

        // Should return for same or shallower search
        let result = tt.probe(key, 3, f32::NEG_INFINITY, f32::INFINITY);
        assert!(result.is_some());
    }
}
