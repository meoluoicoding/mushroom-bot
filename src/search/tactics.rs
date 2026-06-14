use crate::board::Board;
use crate::movegen::RectInfo;
use crate::search::Search;
use crate::types::*;

impl Search {
    #[inline]
    fn lmr_reduction(depth: u8, move_index: usize) -> u8 {
        if depth < 3 || move_index <= 6 {
            return 0;
        }

        let depth_base = match depth {
            0..=2 => 0,
            3 => 1,
            4 => 1,
            5 | 6 => 2,
            _ => 2,
        };

        let move_bonus = if move_index >= 24 {
            2
        } else if move_index >= 14 {
            1
        } else {
            0
        };

        let reduction = depth_base + move_bonus;
        reduction.min(depth.saturating_sub(1)).min(4)
    }

    pub(crate) fn qsearch_plies_left(&self, board: &Board, depth: u8) -> u8 {
        let live_count = board.live_mask.popcount();
        let base = if live_count <= 12 {
            8
        } else if live_count <= 18 {
            7
        } else if live_count <= 26 {
            6
        } else {
            4
        };

        let depth_bonus = if depth <= 1 {
            1
        } else if depth <= 2 && live_count <= 18 {
            1
        } else {
            0
        };

        (base + depth_bonus).min(8)
    }

    pub(crate) fn qsearch_branch_limit(&self, board: &Board) -> usize {
        let live_count = board.live_mask.popcount();
        if live_count <= 12 {
            14
        } else if live_count <= 18 {
            12
        } else if live_count <= 26 {
            10
        } else {
            8
        }
    }

    pub(crate) fn qsearch_should_search(&self, board: &Board, mv: Move, rects: &[RectInfo]) -> bool {
        if self.is_tactical_move(board, mv) {
            return true;
        }

        let see = self.see(board, mv, rects);
        if see >= 0 {
            return true;
        }

        let live_count = board.live_mask.popcount();
        live_count <= 20 && see >= -3
    }

    pub(crate) fn lmr_reduction_for(&self, depth: u8, move_index: usize) -> u8 {
        Self::lmr_reduction(depth, move_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{SearchConfig, Search};

    fn make_rows_with_live_count(live_rows: &[&str], filler: &str) -> Vec<String> {
        let mut rows = vec![filler.to_string(); ROWS];
        for (i, row) in live_rows.iter().enumerate() {
            rows[i] = (*row).to_string();
        }
        rows
    }

    fn make_search() -> Search {
        Search::new(SearchConfig {
            time_budget_ms: 100,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: false,
            use_mcts: false,
            use_qsearch: true,
            use_lmr: true,
            use_futility: false,
            use_mquality: false,
            use_exact_endgame: false,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        })
    }

    fn midgame_board() -> Board {
        let rows = make_rows_with_live_count(
            &[
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
                "12345678912345678",
            ],
            "12345678912345678",
        );
        Board::from_rows(&rows)
    }

    fn endgame_board() -> Board {
        let rows = vec![
            "10000000000000000".to_string(),
            "01000000000000000".to_string(),
            "00100000000000000".to_string(),
            "00010000000000000".to_string(),
            "00001000000000000".to_string(),
            "00000100000000000".to_string(),
            "00000010000000000".to_string(),
            "00000001000000000".to_string(),
            "00000000100000000".to_string(),
            "00000000010000000".to_string(),
        ];
        Board::from_rows(&rows)
    }

    #[test]
    fn test_qsearch_budget_expands_in_endgame() {
        let search = make_search();
        let mid = midgame_board();
        let end = endgame_board();

        let mid_plies = search.qsearch_plies_left(&mid, 2);
        let end_plies = search.qsearch_plies_left(&end, 2);
        let mid_branch = search.qsearch_branch_limit(&mid);
        let end_branch = search.qsearch_branch_limit(&end);

        assert!(end_plies >= mid_plies);
        assert!(end_branch >= mid_branch);
        assert!(end_plies >= 7);
        assert!(end_branch >= 12);
    }

    #[test]
    fn test_lmr_aggressive_late_moves() {
        let search = make_search();

        assert_eq!(search.lmr_reduction_for(2, 20), 0);
        assert_eq!(search.lmr_reduction_for(4, 6), 0);
        assert!(search.lmr_reduction_for(5, 14) >= 2);
        assert!(search.lmr_reduction_for(7, 24) >= 3);
        assert!(search.lmr_reduction_for(8, 40) <= 4);
    }

    #[test]
    fn test_midgame_qsearch_not_short_circuited() {
        let search = make_search();
        let board = midgame_board();
        let qplies = search.qsearch_plies_left(&board, 1);

        assert!(qplies >= 5);
        assert!(search.qsearch_branch_limit(&board) >= 8);
    }

    #[test]
    fn test_qsearch_differs_from_static_eval_on_tactical_midgame() {
        let board = midgame_board();

        let mut static_search = make_search();
        static_search.config.use_qsearch = false;
        let static_value = static_search.negamax(
            &board,
            0,
            0,
            f32::NEG_INFINITY,
            f32::INFINITY,
            0,
            None,
            None,
        );
        let static_nodes = static_search.nodes;

        let mut qsearch = make_search();
        qsearch.config.use_qsearch = true;
        let qsearch_value = qsearch.negamax(
            &board,
            0,
            0,
            f32::NEG_INFINITY,
            f32::INFINITY,
            0,
            None,
            None,
        );
        let qsearch_nodes = qsearch.nodes;

        assert!(qsearch_nodes >= static_nodes);
        assert_ne!(qsearch_value, static_value);
    }
}
