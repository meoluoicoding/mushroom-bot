use crate::board::Board;
use crate::dataloader::{EvalWeights, GameData, RectGeometry};
use crate::types::*;

#[derive(Clone, Copy, Debug)]
pub struct MoveFeatures {
    pub fresh: i32,
    pub recaptured: i32,
    pub own: i32,
    pub live: i32,
    pub area: i32,
    pub edge: i32,
    pub corner: i32,
}

impl Board {
    /// Simple territory-based evaluation
    pub fn fast_eval(&self) -> f32 {
        (self.my_mask.popcount() as i32 - self.opp_mask.popcount() as i32) as f32
    }

    /// Lightweight evaluation for AB leaf nodes (3 terms only)
    pub fn lightweight_evaluate(&self) -> f32 {
        self.lightweight_evaluate_with_weights(&EvalWeights::default())
    }

    pub fn lightweight_evaluate_with_weights(&self, weights: &EvalWeights) -> f32 {
        let my_cells = self.my_mask.popcount() as f32;
        let opp_cells = self.opp_mask.popcount() as f32;

        let my_conn = self.count_connectivity(&self.my_mask) as f32;
        let opp_conn = self.count_connectivity(&self.opp_mask) as f32;

        let (my_corners, my_edges) = self.count_positional(&self.my_mask);
        let (opp_corners, opp_edges) = self.count_positional(&self.opp_mask);

        weights.territory * (my_cells - opp_cells)
            + weights.connectivity * (my_conn - opp_conn)
            + weights.corner_bonus * (my_corners - opp_corners) as f32
            + weights.edge_bonus * (my_edges - opp_edges) as f32
    }

    /// Full 8-term evaluation function with geometry acceleration
    pub fn evaluate(&self, weights: &EvalWeights, game_data: Option<&GameData>) -> f32 {
        let my_cells = self.my_mask.popcount() as f32;
        let opp_cells = self.opp_mask.popcount() as f32;

        let my_conn = self.count_connectivity(&self.my_mask) as f32;
        let opp_conn = self.count_connectivity(&self.opp_mask) as f32;

        let (my_corners, my_edges) = self.count_positional(&self.my_mask);
        let (opp_corners, opp_edges) = self.count_positional(&self.opp_mask);

        let ((my_safe, my_vuln, my_steal, my_mobility, my_threat, my_fork),
            (opp_safe, opp_vuln, opp_steal, opp_mobility, opp_threat, opp_fork)) =
            self.compute_advanced_terms_pair(game_data);

        let threat_penalty = weights.vulnerability.abs() * 1.5;
        let fork_penalty = weights.vulnerability.abs() * 2.5;

        weights.territory * (my_cells - opp_cells)
            + weights.safe_territory * (my_safe - opp_safe)
            + weights.vulnerability * (my_vuln - opp_vuln)
            - threat_penalty * (my_threat - opp_threat)
            - fork_penalty * (my_fork - opp_fork)
            + weights.steal_potential * (my_steal - opp_steal)
            + weights.mobility * (my_mobility - opp_mobility)
            + weights.connectivity * (my_conn - opp_conn)
            + weights.corner_bonus * (my_corners - opp_corners) as f32
            + weights.edge_bonus * (my_edges - opp_edges) as f32
    }

    fn compute_advanced_terms_pair(
        &self,
        game_data: Option<&GameData>,
    ) -> ((f32, f32, f32, f32, f32, f32), (f32, f32, f32, f32, f32, f32)) {
        if let Some(gd) = game_data {
            if !gd.geometry.is_empty() {
                return self.compute_with_geometry_pair(gd);
            }
        }

        let my = self.compute_simple_terms(&self.my_mask, &self.opp_mask);
        let opp = self.compute_simple_terms(&self.opp_mask, &self.my_mask);
        ((my.0, my.1, my.2, my.3, 0.0, 0.0), (opp.0, opp.1, opp.2, opp.3, 0.0, 0.0))
    }

    fn compute_with_geometry_pair(
        &self,
        gd: &GameData,
    ) -> ((f32, f32, f32, f32, f32, f32), (f32, f32, f32, f32, f32, f32)) {
        let mut my_vulnerable = Bitboard::empty();
        let mut opp_vulnerable = Bitboard::empty();
        let mut my_steal_potential = 0f32;
        let mut opp_steal_potential = 0f32;
        let mut mobility = 0f32;

        let mut rect_valid = vec![false; gd.geometry.len()];
        for (idx, rect) in gd.geometry.iter().enumerate() {
            if self.is_rect_valid_with_geometry(rect) {
                rect_valid[idx] = true;
                mobility += 1.0;

                let my_cells = self.my_mask.and(&rect.cell_mask);
                let opp_cells = self.opp_mask.and(&rect.cell_mask);
                let my_in_rect = my_cells.popcount();
                let opp_in_rect = opp_cells.popcount();

                if my_in_rect > 0 {
                    my_vulnerable = my_vulnerable.or(&my_cells);
                }
                if opp_in_rect > 0 {
                    opp_vulnerable = opp_vulnerable.or(&opp_cells);
                }

                if opp_in_rect > 0 && my_in_rect == 0 {
                    my_steal_potential += opp_in_rect as f32;
                }
                if my_in_rect > 0 && opp_in_rect == 0 {
                    opp_steal_potential += my_in_rect as f32;
                }
            }
        }

        let my_vulnerable_count = my_vulnerable.popcount() as f32;
        let opp_vulnerable_count = opp_vulnerable.popcount() as f32;
        let my_safe = self.my_mask.popcount() as f32 - my_vulnerable_count;
        let opp_safe = self.opp_mask.popcount() as f32 - opp_vulnerable_count;

        let mut my_threatened_count = 0f32;
        let mut my_fork_count = 0f32;
        let mut opp_threatened_count = 0f32;
        let mut opp_fork_count = 0f32;

        for i in 0..N_CELLS {
            if self.my_mask.get(i / COLS, i % COLS) {
                let mut threats = 0;
                for &rect_idx in &gd.cell_to_rects[i] {
                    if rect_valid[rect_idx as usize] {
                        threats += 1;
                        if threats >= 2 {
                            break;
                        }
                    }
                }
                if threats >= 1 {
                    my_threatened_count += 1.0;
                }
                if threats >= 2 {
                    my_fork_count += 1.0;
                }
            }

            if self.opp_mask.get(i / COLS, i % COLS) {
                let mut threats = 0;
                for &rect_idx in &gd.cell_to_rects[i] {
                    if rect_valid[rect_idx as usize] {
                        threats += 1;
                        if threats >= 2 {
                            break;
                        }
                    }
                }
                if threats >= 1 {
                    opp_threatened_count += 1.0;
                }
                if threats >= 2 {
                    opp_fork_count += 1.0;
                }
            }
        }

        (
            (my_safe, my_vulnerable_count, my_steal_potential, mobility, my_threatened_count, my_fork_count),
            (opp_safe, opp_vulnerable_count, opp_steal_potential, mobility, opp_threatened_count, opp_fork_count),
        )
    }

    fn is_rect_valid_with_geometry(&self, rect: &RectGeometry) -> bool {
        let live = &self.live_mask;
        let top_ok = !live.and(&rect.top_border).is_empty();
        if !top_ok { return false; }
        let bottom_ok = !live.and(&rect.bottom_border).is_empty();
        if !bottom_ok { return false; }
        let left_ok = !live.and(&rect.left_border).is_empty();
        if !left_ok { return false; }
        let right_ok = !live.and(&rect.right_border).is_empty();
        if !right_ok { return false; }

        let mut sum = 0u32;
        for r in rect.r1..=rect.r2 {
            for c in rect.c1..=rect.c2 {
                sum += self.values[r as usize * COLS + c as usize] as u32;
                if sum > 10 {
                    return false;
                }
            }
        }
        sum == 10
    }

    fn compute_simple_terms(&self, my_mask: &Bitboard, opp_mask: &Bitboard) -> (f32, f32, f32, f32) {
        let opp_count = opp_mask.popcount() as f32;
        let mut my_vuln = 0f32;
        let mut my_safe = 0f32;

        for r in 0..ROWS {
            for c in 0..COLS {
                if my_mask.get(r, c) {
                    if r == 0 || r == ROWS - 1 || c == 0 || c == COLS - 1 {
                        my_safe += 1.0;
                    } else {
                        my_vuln += 0.5;
                        my_safe += 0.5;
                    }
                }
            }
        }

        let live_count = self.live_mask.popcount() as f32;
        let mobility = (live_count / 10.0).max(1.0);
        let steal = opp_count * 0.1;

        (my_safe, my_vuln, steal, mobility)
    }

    pub(crate) fn count_connectivity(&self, mask: &Bitboard) -> i32 {
        let mut conn = 0i32;
        for r in 0..ROWS {
            for c in 0..COLS {
                if mask.get(r, c) {
                    if c + 1 < COLS && mask.get(r, c + 1) {
                        conn += 1;
                    }
                    if r + 1 < ROWS && mask.get(r + 1, c) {
                        conn += 1;
                    }
                }
            }
        }
        conn
    }

    pub(crate) fn count_positional(&self, mask: &Bitboard) -> (i32, i32) {
        let mut corners = 0i32;
        let mut edges = 0i32;

        let corner_positions = [(0, 0), (0, COLS - 1), (ROWS - 1, 0), (ROWS - 1, COLS - 1)];
        for (r, c) in corner_positions {
            if mask.get(r, c) {
                corners += 1;
            }
        }

        for c in 1..COLS - 1 {
            if mask.get(0, c) {
                edges += 1;
            }
            if mask.get(ROWS - 1, c) {
                edges += 1;
            }
        }
        for r in 1..ROWS - 1 {
            if mask.get(r, 0) {
                edges += 1;
            }
            if mask.get(r, COLS - 1) {
                edges += 1;
            }
        }

        (corners, edges)
    }

    pub fn terminal_score(&self) -> f32 {
        let my = self.my_mask.popcount() as i32;
        let opp = self.opp_mask.popcount() as i32;
        let diff = my - opp;

        if diff > 0 {
            1_000_000.0 + diff as f32
        } else if diff < 0 {
            -1_000_000.0 + diff as f32
        } else {
            0.0
        }
    }
}

pub fn compute_move_features(board: &Board, mv: Move) -> MoveFeatures {
    if mv == PASS {
        return MoveFeatures {
            fresh: 0,
            recaptured: 0,
            own: 0,
            live: 0,
            area: 0,
            edge: 0,
            corner: 0,
        };
    }
    let (r1, c1, r2, c2) = mv;
    let opp = opponent(board.player);
    let area = (r2 - r1 + 1) as i32 * (c2 - c1 + 1) as i32;
    let mut fresh = 0;
    let mut recaptured = 0;
    let mut own = 0;
    let mut live = 0;
    let mut edge = 0;
    let mut corner = 0;

    for r in r1 as usize..=r2 as usize {
        for c in c1 as usize..=c2 as usize {
            let idx = cell_index(r, c);
            if board.owners[idx] == 0 {
                fresh += 1;
            } else if board.owners[idx] == opp {
                recaptured += 1;
            } else if board.owners[idx] == board.player {
                own += 1;
            }
            if board.values[idx] > 0 {
                live += 1;
            }

            let is_corner = (r == 0 && c == 0)
                || (r == 0 && c == COLS - 1)
                || (r == ROWS - 1 && c == 0)
                || (r == ROWS - 1 && c == COLS - 1);
            if is_corner {
                corner += 1;
            } else {
                let is_edge = r == 0 || r == ROWS - 1 || c == 0 || c == COLS - 1;
                if is_edge {
                    edge += 1;
                }
            }
        }
    }

    MoveFeatures {
        fresh,
        recaptured,
        own,
        live,
        area,
        edge,
        corner,
    }
}

pub fn static_move_score(features: &MoveFeatures) -> i32 {
    features.recaptured * 120
        + features.fresh * 45
        + features.live * 12
        + features.area * 25
        + features.edge * 8
        + features.corner * 15
        - features.own * 20
}
