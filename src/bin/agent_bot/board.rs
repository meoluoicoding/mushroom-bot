use mushroom_bot::types::*;

#[derive(Clone)]
pub struct AgentBoard {
    pub values: [u8; N_CELLS],
    pub owners: [i8; N_CELLS],
    pub player: i8,
    pub consecutive_passes: i32,
}

impl AgentBoard {
    pub fn from_rows(rows: &[String]) -> Self {
        let mut values = [0u8; N_CELLS];
        for r in 0..ROWS {
            let line = rows[r].trim();
            for (c, ch) in line.chars().enumerate() {
                values[cell_index(r, c)] = ch.to_digit(10).unwrap_or(0) as u8;
            }
        }
        Self { values, owners: [0; N_CELLS], player: FIRST, consecutive_passes: 0 }
    }

    pub fn set_player(&mut self, p: i8) { self.player = p; }

    pub fn apply_move(&mut self, r1: i8, c1: i8, r2: i8, c2: i8) {
        if r1 == -1 {
            self.consecutive_passes += 1;
            self.player = opponent(self.player);
            return;
        }
        for r in r1..=r2 {
            for c in c1..=c2 {
                let idx = cell_index(r as usize, c as usize);
                self.values[idx] = 0;
                self.owners[idx] = self.player;
            }
        }
        self.consecutive_passes = 0;
        self.player = opponent(self.player);
    }

    pub fn make_move(&mut self, r1: i8, c1: i8, r2: i8, c2: i8) -> UndoRecord {
        let record = UndoRecord {
            old_player: self.player,
            old_passes: self.consecutive_passes,
            was_pass: r1 == -1,
            changes: [(0, 0, 0, 0); 170],
            num_changes: 0,
        };
        if r1 == -1 {
            self.consecutive_passes += 1;
            self.player = opponent(self.player);
            return record;
        }
        let mut rec = record;
        for r in r1..=r2 {
            for c in c1..=c2 {
                let idx = cell_index(r as usize, c as usize);
                if rec.num_changes < 170 {
                    rec.changes[rec.num_changes as usize] = (r as u8, c as u8, self.values[idx], self.owners[idx]);
                    rec.num_changes += 1;
                }
                self.values[idx] = 0;
                self.owners[idx] = self.player;
            }
        }
        self.consecutive_passes = 0;
        self.player = opponent(self.player);
        rec
    }

    pub fn unmake_move(&mut self, record: &UndoRecord) {
        self.player = record.old_player;
        self.consecutive_passes = record.old_passes;
        if record.was_pass { return; }
        for i in 0..record.num_changes as usize {
            let (r, c, old_val, old_owner) = record.changes[i];
            let idx = cell_index(r as usize, c as usize);
            self.values[idx] = old_val;
            self.owners[idx] = old_owner;
        }
    }

    pub fn is_legal_move(&self, r1: i8, c1: i8, r2: i8, c2: i8) -> bool {
        if r1 == -1 { return true; }
        if r1 < 0 || r2 >= ROWS as i8 || c1 < 0 || c2 >= COLS as i8 { return false; }
        if r1 > r2 || c1 > c2 { return false; }

        let mut sum: i32 = 0;
        for r in r1..=r2 {
            for c in c1..=c2 {
                let v = self.values[cell_index(r as usize, c as usize)];
                if v > 0 {
                    sum += v as i32;
                    if sum > 10 { return false; }
                }
            }
        }
        if sum != 10 { return false; }

        let (mut top, mut bottom, mut left, mut right) = (false, false, false, false);
        for c in c1..=c2 {
            if self.values[cell_index(r1 as usize, c as usize)] > 0 { top = true; }
            if self.values[cell_index(r2 as usize, c as usize)] > 0 { bottom = true; }
        }
        for r in r1..=r2 {
            if self.values[cell_index(r as usize, c1 as usize)] > 0 { left = true; }
            if self.values[cell_index(r as usize, c2 as usize)] > 0 { right = true; }
        }
        top && bottom && left && right
    }

    pub fn owned_cells(&self, p: i8) -> i32 {
        self.owners.iter().filter(|&&o| o == p).count() as i32
    }

    pub fn is_terminal(&self) -> bool {
        self.consecutive_passes >= 2
    }

    pub fn terminal_score_for(&self, p: i8) -> i32 {
        let margin = self.owned_cells(p) - self.owned_cells(opponent(p));
        if margin > 0 { 100000 + margin } else if margin < 0 { -100000 + margin } else { 0 }
    }
}

pub struct UndoRecord {
    old_player: i8,
    old_passes: i32,
    was_pass: bool,
    changes: [(u8, u8, u8, i8); 170],
    num_changes: i32,
}

impl Default for UndoRecord {
    fn default() -> Self {
        Self {
            old_player: 0,
            old_passes: 0,
            was_pass: false,
            changes: [(0, 0, 0, 0); 170],
            num_changes: 0,
        }
    }
}
