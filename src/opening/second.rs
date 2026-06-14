use crate::board::Board;
use crate::dataloader::{EvalWeights, GameData};
use crate::search::SearchConfig;
use crate::timeman::SearchPhase;
use crate::types::*;

use super::stealth;
use super::traps;

#[derive(Clone, Copy, Debug, Default)]
pub struct SecondSideTuning;

impl SecondSideTuning {
    const TRAP_MIN_LIVE: u32 = 14;
    const TRAP_MAX_LIVE: u32 = 38;

    pub fn name(self) -> &'static str {
        "second"
    }

    pub fn clear_tt_on_init(self) -> bool {
        true
    }

    pub fn search_config(self, mut config: SearchConfig, phase: SearchPhase) -> SearchConfig {
        config.use_aspiration = matches!(
            phase,
            SearchPhase::MidgameFull | SearchPhase::MidgameConserve | SearchPhase::Endgame
        );
        config.use_mcts = false;
        config
    }

    pub fn trap_bonus(
        self,
        board: &Board,
        mv: Move,
        child: &Board,
        weights: &EvalWeights,
        game_data: Option<&GameData>,
    ) -> f32 {
        if mv == PASS {
            return -10_000.0;
        }

        let live = board.live_mask.popcount();
        if !(Self::TRAP_MIN_LIVE..=Self::TRAP_MAX_LIVE).contains(&live) {
            return 0.0;
        }

        let book_bonus = traps::trapbook_bonus(board, mv, child, weights, game_data);
        let tactical_bonus = traps::tactical_squeeze_bonus(child, weights, game_data);

        book_bonus + tactical_bonus
    }

    pub fn root_move_bonus(
        self,
        board: &Board,
        mv: Move,
        child: &Board,
        weights: &EvalWeights,
        game_data: Option<&GameData>,
    ) -> f32 {
        if mv == PASS {
            return -5000.0;
        }

        let opponent_pressure = child.lightweight_evaluate_with_weights(weights);
        let mobility = child.live_mask.popcount() as f32;
        let edge_bias = if mv.0 == 0
            || mv.2 as usize == crate::types::ROWS - 1
            || mv.1 == 0
            || mv.3 as usize == crate::types::COLS - 1
        {
            12.0
        } else {
            0.0
        };
        let trap_bonus = self.trap_bonus(board, mv, child, weights, game_data);
        let stealth_bonus = stealth::stealth_bonus(board, mv, child);

        let live = board.live_mask.popcount();
        let mut opening_bonus = 0.0f32;
        if live > 34 {
            let (r1, c1, r2, c2) = mv;
            let area = ((r2 - r1 + 1) * (c2 - c1 + 1)) as f32;
            opening_bonus += area * 1.5;

            let opp = crate::types::opponent(board.player);
            let mut corner = 0f32;
            let mut edge = 0f32;
            let mut recaptured = 0f32;
            let mut own = 0f32;
            for r in r1 as usize..=r2 as usize {
                for c in c1 as usize..=c2 as usize {
                    let idx = crate::types::cell_index(r, c);
                    if board.owners[idx] == opp {
                        recaptured += 1.0;
                    } else if board.owners[idx] == board.player {
                        own += 1.0;
                    }
                    let is_corner = (r == 0 && c == 0)
                        || (r == 0 && c == crate::types::COLS - 1)
                        || (r == crate::types::ROWS - 1 && c == 0)
                        || (r == crate::types::ROWS - 1 && c == crate::types::COLS - 1);
                    if is_corner {
                        corner += 1.0;
                    } else if r == 0 || r == crate::types::ROWS - 1 || c == 0 || c == crate::types::COLS - 1 {
                        edge += 1.0;
                    }
                }
            }
            opening_bonus += corner * 18.0;
            opening_bonus += edge * 6.0;
            opening_bonus += recaptured * 30.0;
            opening_bonus -= own * 6.0;
        }

        (-opponent_pressure * 0.24) + edge_bias - (mobility * 0.02) + trap_bonus + stealth_bonus + opening_bonus
    }
}
