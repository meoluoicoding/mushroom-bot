use crate::board::Board;
use crate::dataloader::{EvalWeights, GameData};
use crate::search::SearchConfig;
use crate::timeman::SearchPhase;
use crate::types::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct FirstSideTuning;

impl FirstSideTuning {
    pub fn name(self) -> &'static str {
        "first"
    }

    pub fn clear_tt_on_init(self) -> bool {
        true
    }

    pub fn search_config(self, mut config: SearchConfig, phase: SearchPhase) -> SearchConfig {
        config.use_aspiration = matches!(phase, SearchPhase::MidgameFull | SearchPhase::Endgame);
        config.use_mcts = false;
        config
    }

    pub fn root_move_bonus(
        self,
        board: &Board,
        mv: Move,
        child: &Board,
        _weights: &EvalWeights,
        _game_data: Option<&GameData>,
    ) -> f32 {
        if mv == PASS {
            return -5000.0;
        }

        let live = board.live_mask.popcount();
        if live < 24 {
            return 0.0;
        }

        let (r1, c1, r2, c2) = mv;
        let area = ((r2 - r1 + 1) * (c2 - c1 + 1)) as f32;

        let opp = crate::types::opponent(board.player);
        let mut recaptured = 0f32;
        let mut fresh = 0f32;
        let mut own = 0f32;
        let mut corner = 0f32;
        let mut edge = 0f32;

        for r in r1 as usize..=r2 as usize {
            for c in c1 as usize..=c2 as usize {
                let idx = crate::types::cell_index(r, c);
                if board.owners[idx] == 0 {
                    fresh += 1.0;
                } else if board.owners[idx] == opp {
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
                } else {
                    let is_edge = r == 0 || r == crate::types::ROWS - 1 || c == 0 || c == crate::types::COLS - 1;
                    if is_edge {
                        edge += 1.0;
                    }
                }
            }
        }

        let opponent_pressure = child.lightweight_evaluate_with_weights(_weights);
        let child_live = child.live_mask.popcount() as f32;

        let mut bonus = 0.0f32;
        bonus += corner * 30.0;
        bonus += edge * 12.0;
        bonus += area * 5.0;
        bonus += recaptured * 45.0;
        bonus += fresh * 8.0;
        bonus -= own * 12.0;
        bonus -= opponent_pressure * 0.10;
        bonus -= child_live * 0.01;

        if area >= 6.0 {
            bonus += area * 1.5;
        }

        if recaptured > 0.0 {
            bonus += 18.0;
        }

        bonus
    }
}
