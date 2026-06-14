use crate::board::Board;
use crate::search::SearchConfig;
use crate::timeman::SearchPhase;
use crate::types::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameSide {
    First,
    Second,
}

impl GameSide {
    pub fn from_player(player: i8) -> Self {
        if player == crate::types::FIRST {
            Self::First
        } else {
            Self::Second
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::First => "FIRST",
            Self::Second => "SECOND",
        }
    }

    pub fn tuning(self) -> SideTuning {
        match self {
            Self::First => SideTuning::First(super::first::FirstSideTuning::default()),
            Self::Second => SideTuning::Second(super::second::SecondSideTuning::default()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SideTuning {
    First(super::first::FirstSideTuning),
    Second(super::second::SecondSideTuning),
}

impl SideTuning {
    pub fn name(self) -> &'static str {
        match self {
            Self::First(t) => t.name(),
            Self::Second(t) => t.name(),
        }
    }

    pub fn search_config(self, config: SearchConfig, phase: SearchPhase) -> SearchConfig {
        match self {
            Self::First(t) => t.search_config(config, phase),
            Self::Second(t) => t.search_config(config, phase),
        }
    }

    pub fn clear_tt_on_init(self) -> bool {
        match self {
            Self::First(t) => t.clear_tt_on_init(),
            Self::Second(t) => t.clear_tt_on_init(),
        }
    }

    pub fn root_move_bonus(
        self,
        board: &Board,
        mv: Move,
        child: &Board,
        weights: &crate::dataloader::EvalWeights,
        game_data: Option<&crate::dataloader::GameData>,
    ) -> f32 {
        match self {
            Self::First(t) => t.root_move_bonus(board, mv, child, weights, game_data),
            Self::Second(t) => t.root_move_bonus(board, mv, child, weights, game_data),
        }
    }
}
