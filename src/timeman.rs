pub enum SearchPhase {
    /// ≤12 mushrooms: solve exactly with max depth
    Endgame,
    /// ≥2500ms remaining: full iterative deepening
    MidgameFull,
    /// 500-2500ms remaining: conservative depth
    MidgameConserve,
    /// <500ms remaining: emergency heuristic
    Emergency,
}

pub struct TimeManager {
    pub my_time_left_ms: u64,
    pub opp_time_left_ms: u64,
    pub reserve_ms: u64,
}

impl TimeManager {
    pub fn new() -> Self {
        Self {
            my_time_left_ms: 10_000,
            opp_time_left_ms: 10_000,
            reserve_ms: 200,
        }
    }

    pub fn update(&mut self, my_ms: u64, opp_ms: u64) {
        self.my_time_left_ms = my_ms;
        self.opp_time_left_ms = opp_ms;
    }

    pub fn phase(&self, live_count: u32, legal_moves: usize) -> SearchPhase {
        if live_count <= 12 || legal_moves <= 10 {
            SearchPhase::Endgame
        } else if self.my_time_left_ms < 500 {
            SearchPhase::Emergency
        } else if self.my_time_left_ms < 2500 {
            SearchPhase::MidgameConserve
        } else {
            SearchPhase::MidgameFull
        }
    }

    pub fn search_budget_ms(&self, live_count: u32, legal_moves: usize) -> u64 {
        let usable = self.my_time_left_ms.saturating_sub(self.reserve_ms);

        if live_count <= 12 || legal_moves <= 10 {
            return ((usable * 60) / 100).max(10);
        }

        if self.my_time_left_ms < 500 {
            return (self.my_time_left_ms / 5).clamp(50, 100);
        }

        let moves_left = if live_count > 60 {
            22u64
        } else if live_count > 40 {
            17
        } else if live_count > 25 {
            12
        } else if live_count > 15 {
            8
        } else {
            5
        };

        let budget = (usable / moves_left).clamp(50, 800);

        if self.my_time_left_ms < 2500 {
            budget.clamp(50, 400)
        } else {
            budget
        }
    }
}
