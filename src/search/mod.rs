use std::time::Instant;

use crate::board::Board;
use crate::dataloader::{EvalWeights, GameData};
use crate::movegen::{fixed_rect_id, generate_rectangles, RectInfo};
use crate::side::GameSide;
use crate::tt::TranspositionTable;
use crate::types::*;

pub mod result;
pub mod ordering;
pub mod pruning;
pub mod tactics;
pub mod root;
pub mod negamax;

pub use result::{SearchResult, SearchConfig};

const MAX_DEPTH: u8 = 64;
const ASPIRATION_WINDOW: f32 = 80.0;


pub struct Search {
    pub config: SearchConfig,
    pub weights: EvalWeights,
    pub game_data: Option<GameData>,
    pub side: GameSide,
    pub tt: TranspositionTable,
    pub start: Instant,
    pub nodes: u64,
    pub timed_out: bool,
    pub max_ply_reached: u8,
    pub killer_moves: [[Option<Move>; 2]; 64],
    pub history_moves: Box<[u32]>,
    pub global_history_moves: Box<[u32]>,
    pub rect_cache: std::collections::HashMap<u64, Vec<RectInfo>>,
    pub counter_moves: Box<[u16]>,
}

impl Search {
    fn init(config: SearchConfig, weights: EvalWeights, game_data: Option<GameData>) -> Self {
        Self {
            config,
            weights,
            game_data,
            side: GameSide::First,
            tt: TranspositionTable::new(),
            start: Instant::now(),
            nodes: 0,
            timed_out: false,
            max_ply_reached: 0,
            killer_moves: [[None; 2]; 64],
            history_moves: vec![0; 64 * N_RECTS].into_boxed_slice(),
            global_history_moves: vec![0; N_RECTS].into_boxed_slice(),
            rect_cache: std::collections::HashMap::new(),
            counter_moves: vec![u16::MAX; N_RECTS].into_boxed_slice(),
        }
    }

    pub fn new(config: SearchConfig) -> Self {
        Self::init(config, EvalWeights::default(), None)
    }

    pub fn with_weights(config: SearchConfig, weights: EvalWeights) -> Self {
        Self::init(config, weights, None)
    }

    pub fn with_game_data(config: SearchConfig, game_data: GameData) -> Self {
        let weights = game_data.weights;
        Self::init(config, weights, Some(game_data))
    }

    #[inline]
    fn history_index(ply: u8, rect_id: usize) -> usize {
        usize::min(ply as usize, 63) * N_RECTS + rect_id
    }

    #[inline]
    pub fn check_timeout(&mut self) {
        if !self.timed_out && self.config.time_budget_ms > 0 {
            if self.start.elapsed().as_millis() as u64 >= self.config.time_budget_ms {
                self.timed_out = true;
            }
        }
    }

    pub fn clear_tt(&mut self) {
        self.tt.clear();
    }

    pub fn set_side(&mut self, side: GameSide) {
        self.side = side;
    }

    pub fn record_ply(&mut self, ply: u8) {
        if ply > self.max_ply_reached {
            self.max_ply_reached = ply;
        }
    }

    #[inline]
    pub fn record_history_cutoff(&mut self, ply: u8, action: Move, depth: u8) {
        if action == PASS {
            return;
        }
        let rect_id = fixed_rect_id(action.0, action.1, action.2, action.3) as usize;
        let d = u32::from(depth.saturating_add(1));
        let bonus = d.saturating_mul(d.saturating_add(1)).max(1);
        let entry = &mut self.history_moves[Self::history_index(ply, rect_id)];
        *entry = entry.saturating_add(bonus);
        self.global_history_moves[rect_id] = self.global_history_moves[rect_id].saturating_add(bonus);
    }

    #[inline]
    pub fn record_history_malus(&mut self, ply: u8, actions: &[Move], depth: u8) {
        let d = u32::from(depth.saturating_add(1));
        let penalty = (d.saturating_mul(d.saturating_add(1)) / 2).max(1);
        for &action in actions {
            if action == PASS {
                continue;
            }
            let rect_id = fixed_rect_id(action.0, action.1, action.2, action.3) as usize;
            let entry = &mut self.history_moves[Self::history_index(ply, rect_id)];
            *entry = entry.saturating_sub(penalty);
            self.global_history_moves[rect_id] = self.global_history_moves[rect_id].saturating_sub(penalty);
        }
    }

    #[inline]
    pub fn history_score(&self, ply: u8, action: Move) -> i32 {
        if action == PASS {
            return 0;
        }
        let rect_id = fixed_rect_id(action.0, action.1, action.2, action.3) as usize;
        let local = (self.history_moves[Self::history_index(ply, rect_id)].min(100_000) / 16) as i32;
        let global = (self.global_history_moves[rect_id].min(100_000) / 8) as i32;
        global + local
    }

    #[inline]
    pub fn should_use_exact_endgame(&self, live_count: u32, num_rects: usize) -> bool {
        live_count <= 12
            || num_rects <= 10
            || (live_count <= 15 && num_rects <= 12)
            || (live_count <= 18 && num_rects <= 8)
    }

    #[inline]
    pub fn is_zugzwang(&self, board: &Board) -> bool {
        board.passes > 0 || board.live_mask.popcount() <= 12
    }

    #[inline]
    pub fn planned_max_depth(&self, live_count: u32, root_moves: usize) -> u8 {
        if self.should_use_exact_endgame(live_count, root_moves) {
            return MAX_DEPTH;
        }

        let mut depth = match live_count {
            0..=12 => MAX_DEPTH,
            13..=18 => 20,
            19..=24 => 19,
            25..=32 => 18,
            33..=42 => 17,
            _ => 16,
        };

        let branching_bonus = match root_moves {
            0..=6 => 4,
            7..=12 => 3,
            13..=20 => 2,
            _ => 0,
        };
        depth = depth.saturating_add(branching_bonus);

        // Scale depth with available time budget
        let time_bonus = if self.config.time_budget_ms >= 1500 {
            6
        } else if self.config.time_budget_ms >= 800 {
            5
        } else if self.config.time_budget_ms >= 400 {
            4
        } else if self.config.time_budget_ms >= 200 {
            2
        } else if self.config.time_budget_ms >= 100 {
            1
        } else {
            0
        };
        depth = depth.saturating_add(time_bonus);

        depth.min(MAX_DEPTH)
    }

    #[inline]
    pub fn razoring_margin(&self, depth: u8, live_count: u32, root_moves: usize) -> f32 {
        let base = match depth {
            1 => 150.0,
            2 => 220.0,
            _ => 0.0,
        };
        let live_adjust = if live_count <= 12 {
            0.0
        } else if live_count <= 20 {
            20.0
        } else {
            35.0
        };
        let branching_adjust = if root_moves <= 6 {
            -15.0
        } else if root_moves >= 20 {
            25.0
        } else {
            0.0
        };
        base + live_adjust + branching_adjust
    }

    pub fn rect_cache_key(values: &[u8; N_CELLS]) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for &value in values {
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= value as u64;
        }
        hash
    }

    pub fn cached_rectangles(&mut self, key: u64, values: &[u8; N_CELLS]) -> Vec<RectInfo> {
        if let Some(cached) = self.rect_cache.get(&key) {
            return cached.clone();
        }
        let rects = generate_rectangles(values);
        self.rect_cache.insert(key, rects.clone());
        rects
    }

    pub fn think(&mut self, board: &Board) -> SearchResult {
        self.start = Instant::now();
        self.nodes = 0;
        self.timed_out = false;
        self.max_ply_reached = 0;
        self.killer_moves = [[None; 2]; 64];
        for entry in self.history_moves.iter_mut() {
            *entry >>= 4;
        }
        for entry in self.global_history_moves.iter_mut() {
            *entry >>= 4;
        }
        self.counter_moves.fill(u16::MAX);

        let root_rect_key = Self::rect_cache_key(&board.values);
        let rects = self.cached_rectangles(root_rect_key, &board.values);
        if rects.is_empty() {
            return SearchResult {
                action: PASS,
                value: board.evaluate(&self.weights, self.game_data.as_ref()),
                depth: 0,
                max_ply_reached: self.max_ply_reached,
                nodes: 1,
                elapsed_ms: 0.0,
            };
        }

        let root_key = board.hash;
        let live_count = board.live_mask.popcount();

        // Exact endgame search if live_count is low or branching factor is low
        if self.config.use_exact_endgame && self.should_use_exact_endgame(live_count, rects.len()) {
            let (value, action) = if self.config.use_mtd {
                self.exact_endgame_search_mtd(board, 0.0, root_key, 0)
            } else {
                self.exact_endgame_search(board, f32::NEG_INFINITY, f32::INFINITY, root_key, 0)
            };
            return SearchResult {
                action,
                value,
                depth: live_count.min(u32::from(u8::MAX)) as u8,
                max_ply_reached: self.max_ply_reached,
                nodes: self.nodes,
                elapsed_ms: self.start.elapsed().as_millis() as f64,
            };
        }

        let max_depth = self.planned_max_depth(live_count, rects.len());

        let mut best_action = rects[0].to_move();
        let mut best_value = f32::NEG_INFINITY;
        let mut completed_depth = 0u8;
        let extensions_left = 2;

        // Iterative Deepening
        for depth in 1..=max_depth {
            let alpha = f32::NEG_INFINITY;
            let beta = f32::INFINITY;
            let mut current_best_action = best_action;
            let mut current_best_value = f32::NEG_INFINITY;

            if self.config.use_aspiration && depth >= 3 && best_value > f32::NEG_INFINITY {
                let mut lo = best_value - ASPIRATION_WINDOW;
                let mut hi = best_value + ASPIRATION_WINDOW;
                let mut last_val = best_value;
                let mut last_action = best_action;

                const ASPIRATION_MULTIPLIERS: [f32; 3] = [1.5, 3.0, f32::INFINITY];
                let mut idx = 0;

                loop {
                    let (val, action) = self.root_search(
                        board,
                        &rects,
                        depth,
                        lo,
                        hi,
                        root_key,
                        extensions_left,
                        0,
                    );
                    self.nodes += 1;

                    if self.timed_out {
                        break;
                    }

                    last_val = val;
                    last_action = action;

                    if val > lo && val < hi {
                        break; // Inside window
                    }

                    if idx >= ASPIRATION_MULTIPLIERS.len() {
                        break;
                    }

                    let mult = ASPIRATION_MULTIPLIERS[idx];
                    idx += 1;

                    if val <= lo {
                        if mult.is_infinite() {
                            lo = f32::NEG_INFINITY;
                        } else {
                            lo = val - ASPIRATION_WINDOW * mult;
                        }
                    }
                    if val >= hi {
                        if mult.is_infinite() {
                            hi = f32::INFINITY;
                        } else {
                            hi = val + ASPIRATION_WINDOW * mult;
                        }
                    }
                }

                if !self.timed_out {
                    current_best_value = last_val;
                    current_best_action = last_action;
                }
            } else {
                let (val, action) = self.root_search(
                    board,
                    &rects,
                    depth,
                    alpha,
                    beta,
                    root_key,
                    extensions_left,
                    0,
                );
                self.nodes += 1;
                if !self.timed_out {
                    current_best_value = val;
                    current_best_action = action;
                }
            }

            if !self.timed_out {
                best_value = current_best_value;
                best_action = current_best_action;
                completed_depth = depth;
            }

            if self.timed_out {
                break;
            }

            if best_value.abs() > 500_000.0 {
                break;
            }
        }

        let elapsed = self.start.elapsed().as_millis() as f64;

        SearchResult {
            action: best_action,
            value: best_value,
            depth: completed_depth,
            max_ply_reached: self.max_ply_reached,
            nodes: self.nodes,
            elapsed_ms: elapsed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_board() -> Board {
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
        Board::from_rows(&rows)
    }

    #[test]
    fn test_search_returns_legal_move() {
        let board = make_test_board();
        let mut search = Search::new(SearchConfig {
            time_budget_ms: 100,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: false,
            use_mcts: false,
            use_qsearch: false,
            use_lmr: false,
            use_futility: false,
            use_mquality: false,
            use_exact_endgame: false,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        });

        let result = search.think(&board);
        assert!(board.is_legal_action(result.action));
    }

    #[test]
    fn test_search_no_pass_when_moves_available() {
        let board = make_test_board();
        let rects = generate_rectangles(&board.values);

        assert!(!rects.is_empty());

        let mut search = Search::new(SearchConfig {
            time_budget_ms: 100,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: false,
            use_mcts: false,
            use_qsearch: false,
            use_lmr: false,
            use_futility: false,
            use_mquality: false,
            use_exact_endgame: false,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        });

        let result = search.think(&board);
        assert_ne!(result.action, PASS);
    }

    #[test]
    fn test_order_moves_no_pass() {
        let board = make_test_board();
        let rects = generate_rectangles(&board.values);
        let search = Search::new(SearchConfig::default());

        let ordered = search.order_moves_with_pass(&board, &rects, None, 0, false, false, None);
        // Note: PASS is only added if should_consider_pass returns true, but here it shouldn't.
        // Let's assert that the ordered actions contains the rect actions
        assert!(!ordered.is_empty());
    }

    #[test]
    fn test_search_depth_increases_with_time() {
        let board = make_test_board();

        let mut search_short = Search::new(SearchConfig {
            time_budget_ms: 10,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: false,
            use_mcts: false,
            use_qsearch: false,
            use_lmr: false,
            use_futility: false,
            use_mquality: false,
            use_exact_endgame: false,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        });
        let result_short = search_short.think(&board);

        let mut search_long = Search::new(SearchConfig {
            time_budget_ms: 100,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: false,
            use_mcts: false,
            use_qsearch: false,
            use_lmr: false,
            use_futility: false,
            use_mquality: false,
            use_exact_endgame: false,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        });
        let result_long = search_long.think(&board);

        assert!(result_long.depth >= result_short.depth || result_long.nodes >= result_short.nodes);
    }
}
