use crate::types::Move;

#[derive(Clone, Copy, Debug)]
pub struct SearchConfig {
    pub time_budget_ms: u64,
    pub use_tt: bool,
    pub use_ordering: bool,
    pub use_second_bonus: bool,
    pub use_aspiration: bool,
    pub use_mcts: bool,
    pub use_qsearch: bool,
    pub use_lmr: bool,
    pub use_futility: bool,
    pub use_mquality: bool,
    pub use_exact_endgame: bool,
    pub use_nmp: bool,
    pub use_singular_extension: bool,
    pub use_mtd: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            time_budget_ms: 100,
            use_tt: true,
            use_ordering: true,
            use_second_bonus: true,
            use_aspiration: true,
            use_mcts: false, // Default false as per spec
            use_qsearch: false, // Default false as per spec
            use_lmr: true,
            use_futility: false,
            use_mquality: false,
            use_exact_endgame: true,
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: false,
        }
    }
}

pub struct SearchResult {
    pub action: Move,
    pub value: f32,
    pub depth: u8,
    pub max_ply_reached: u8,
    pub nodes: u64,
    pub elapsed_ms: f64,
}
