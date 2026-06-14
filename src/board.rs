
use crate::types::*;

#[derive(Clone, Debug)]
pub struct Board {
    pub values: [u8; N_CELLS],
    pub owners: [i8; N_CELLS],
    pub player: i8,
    pub passes: u8,
    pub hash: u64,

    // Cached bitboards
    pub my_mask: Bitboard,
    pub opp_mask: Bitboard,
    pub live_mask: Bitboard,
    pub empty_mask: Bitboard,
}

impl Board {
    pub fn from_rows(rows: &[String]) -> Self {
        let mut values = [0u8; N_CELLS];
        let owners = [0i8; N_CELLS];
        for r in 0..ROWS {
            let line = rows[r].trim();
            for (c, ch) in line.chars().enumerate() {
                let idx = cell_index(r, c);
                values[idx] = ch.to_digit(10).unwrap_or(0) as u8;
            }
        }
        Self::from_parts(values, owners, FIRST, 0)
    }

    pub fn from_tool_board(board: &[Vec<i32>]) -> Self {
        let mut values = [0u8; N_CELLS];
        let mut owners = [0i8; N_CELLS];
        for r in 0..ROWS {
            for c in 0..COLS {
                let idx = cell_index(r, c);
                let cell = board[r][c];
                if cell > 0 {
                    values[idx] = cell as u8;
                } else if cell == -1 {
                    owners[idx] = FIRST;
                } else if cell == -2 {
                    owners[idx] = SECOND;
                }
            }
        }
        Self::from_parts(values, owners, FIRST, 0)
    }

    pub fn from_parts(values: [u8; N_CELLS], owners: [i8; N_CELLS], player: i8, passes: u8) -> Self {
        let hash = crate::tt::hash_board(&values, &owners, player, passes);
        let mut b = Self {
            values,
            owners,
            player,
            passes,
            hash,
            my_mask: Bitboard::empty(),
            opp_mask: Bitboard::empty(),
            live_mask: Bitboard::empty(),
            empty_mask: Bitboard::empty(),
        };
        b.rebuild_bitboards();
        b
    }

    fn rebuild_bitboards(&mut self) {
        self.my_mask = Bitboard::empty();
        self.opp_mask = Bitboard::empty();
        self.live_mask = Bitboard::empty();
        self.empty_mask = Bitboard::empty();
        for i in 0..N_CELLS {
            let r = i / COLS;
            let c = i % COLS;
            match self.owners[i] {
                o if o == self.player => self.my_mask.set(r, c),
                o if o == opponent(self.player) => self.opp_mask.set(r, c),
                _ => self.empty_mask.set(r, c),
            }
            if self.values[i] > 0 {
                self.live_mask.set(r, c);
            }
        }
    }

    #[inline]
    pub fn current_player(&self) -> i8 {
        self.player
    }

    #[inline]
    pub fn is_terminal(&self) -> bool {
        self.passes >= 2
    }

    pub fn apply_action(&self, action: Move) -> Self {
        let (r1, c1, r2, c2) = action;
        let next_player = opponent(self.player);
        let next_passes = if action == PASS { self.passes + 1 } else { 0 };

        let hash = crate::tt::hash_update(
            self.hash,
            &self.values,
            &self.owners,
            action,
            self.player,
            self.passes,
            next_passes,
        );

        if action == PASS {
            return Self {
                values: self.values,
                owners: self.owners,
                player: next_player,
                passes: next_passes,
                hash,
                my_mask: self.opp_mask,
                opp_mask: self.my_mask,
                live_mask: self.live_mask,
                empty_mask: self.empty_mask,
            };
        }

        let mut values = self.values;
        let mut owners = self.owners;
        let mut my_mask = self.my_mask;
        let mut opp_mask = self.opp_mask;
        let mut live_mask = self.live_mask;
        let mut empty_mask = self.empty_mask;
        let opp = opponent(self.player);

        for r in r1 as usize..=r2 as usize {
            for c in c1 as usize..=c2 as usize {
                let idx = cell_index(r, c);
                let old_owner = owners[idx];

                if old_owner == opp {
                    opp_mask.clear(r, c);
                } else if old_owner == 0 {
                    empty_mask.clear(r, c);
                }
                my_mask.set(r, c);

                if values[idx] > 0 {
                    live_mask.clear(r, c);
                    values[idx] = 0;
                }
                owners[idx] = self.player;
            }
        }

        Self {
            values,
            owners,
            player: next_player,
            passes: next_passes,
            hash,
            my_mask: opp_mask,
            opp_mask: my_mask,
            live_mask,
            empty_mask,
        }
    }

    #[inline]
    pub fn score(&self, player_id: i8) -> i32 {
        if player_id == self.player {
            self.my_mask.popcount() as i32
        } else {
            self.opp_mask.popcount() as i32
        }
    }

    #[inline]
    pub fn is_legal_action(&self, mv: Move) -> bool {
        mv == PASS || self.is_valid_rect(mv)
    }

    pub fn is_valid_rect(&self, mv: Move) -> bool {
        crate::movegen::is_valid_rectangle(&self.values, mv)
    }

    /// Score a move for ordering purposes
    pub fn action_score(&self, mv: Move) -> (i32, i32, i32, i32, i32, i32) {
        if mv == PASS {
            return (-10_000, 0, 0, 0, 0, 0);
        }
        let (r1, c1, r2, c2) = mv;
        let opp = opponent(self.player);
        let area = ((r2 - r1 + 1) * (c2 - c1 + 1)) as i32;
        let mut fresh = 0i32;
        let mut recaptured = 0i32;
        let mut own = 0i32;
        let mut live = 0i32;
        for r in r1 as usize..=r2 as usize {
            for c in c1 as usize..=c2 as usize {
                let idx = cell_index(r, c);
                if self.owners[idx] == 0 {
                    fresh += 1;
                } else if self.owners[idx] == opp {
                    recaptured += 1;
                } else {
                    own += 1;
                }
                if self.values[idx] > 0 {
                    live += 1;
                }
            }
        }
        (fresh + 2 * recaptured, recaptured, fresh, live, area - own, area)
    }

    #[inline]
    pub fn rectangles_overlap(a: Move, b: Move) -> bool {
        if a == PASS || b == PASS {
            return false;
        }
        let (ar1, ac1, ar2, ac2) = a;
        let (br1, bc1, br2, bc2) = b;
        !(ar2 < br1 || br2 < ar1 || ac2 < bc1 || bc2 < ac1)
    }

    pub fn rectangle_exchange_eval(&self, mv: Move, reply_depth: u8) -> i32 {
        if mv == PASS || !self.is_legal_action(mv) {
            return i32::MIN / 4;
        }

        let immediate = self.action_score(mv).0;
        if reply_depth == 0 {
            return immediate;
        }

        let child = self.apply_action(mv);
        let reply_penalty = child.best_overlap_exchange_gain(mv, reply_depth - 1);
        immediate - reply_penalty
    }

    fn best_overlap_exchange_gain(&self, anchor: Move, reply_depth: u8) -> i32 {
        if reply_depth == 0 {
            return 0;
        }

        let replies = crate::movegen::generate_rectangles(&self.values);
        let mut best = 0i32;

        for reply in replies {
            let reply_mv = reply.to_move();
            if !Self::rectangles_overlap(reply_mv, anchor) {
                continue;
            }

            let immediate = self.action_score(reply_mv).0;
            let child = self.apply_action(reply_mv);
            let net = immediate - child.best_overlap_exchange_gain(reply_mv, reply_depth - 1);
            if net > best {
                best = net;
            }
        }

        best.max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_creation() {
        let rows: Vec<String> = vec![
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
        ];
        let board = Board::from_rows(&rows);
        assert_eq!(board.player, FIRST);
        assert_eq!(board.passes, 0);
        assert!(!board.is_terminal());
    }

    #[test]
    fn test_apply_pass() {
        let rows: Vec<String> = vec![
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
        ];
        let board = Board::from_rows(&rows);
        let after_pass = board.apply_action(PASS);
        assert_eq!(after_pass.player, SECOND);
        assert_eq!(after_pass.passes, 1);
        assert!(!after_pass.is_terminal());

        let after_two_passes = after_pass.apply_action(PASS);
        assert!(after_two_passes.is_terminal());
    }

    #[test]
    fn test_fast_eval_initial() {
        let rows: Vec<String> = vec![
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
        ];
        let board = Board::from_rows(&rows);
        // Initially no cells owned, eval should be 0
        assert_eq!(board.fast_eval(), 0.0);
    }

    #[test]
    fn test_bitboard_operations() {
        let mut bb = Bitboard::empty();
        assert!(bb.is_empty());
        assert_eq!(bb.popcount(), 0);

        bb.set(0, 0);
        assert!(!bb.is_empty());
        assert!(bb.get(0, 0));
        assert!(!bb.get(0, 1));
        assert_eq!(bb.popcount(), 1);

        bb.set(5, 10);
        assert_eq!(bb.popcount(), 2);

        bb.clear(0, 0);
        assert_eq!(bb.popcount(), 1);
        assert!(!bb.get(0, 0));
    }

    #[test]
    fn test_connectivity() {
        let rows: Vec<String> = vec![
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
        ];
        let board = Board::from_rows(&rows);

        // Create a simple mask with adjacent cells
        let mut mask = Bitboard::empty();
        mask.set(0, 0);
        mask.set(0, 1);
        mask.set(1, 0);

        // Should have 2 connections: (0,0)-(0,1) and (0,0)-(1,0)
        let conn = board.count_connectivity(&mask);
        assert_eq!(conn, 2);
    }
}
