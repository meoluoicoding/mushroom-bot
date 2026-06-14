pub enum GamePhase {
    EndgameExact,
    MidgameFull,
    MidgameConserve,
    Emergency,
}

pub struct TimeManager {
    pub phase: GamePhase,
    pub optimum_time_ms: i32,
    pub max_depth: i32,
    pub use_aspiration: bool,
}

impl TimeManager {
    const CONSERVE_MS: i32 = 2500;
    const EMERGENCY_MS: i32 = 500;
    const RESERVE_MS: i32 = 500;

    pub fn new() -> Self {
        Self {
            phase: GamePhase::MidgameFull,
            optimum_time_ms: 100,
            max_depth: 6,
            use_aspiration: true,
        }
    }

    pub fn init(&mut self, my_time_ms: i32, live_mushrooms: i32) {
        let remaining = (my_time_ms - Self::RESERVE_MS).max(50);

        if live_mushrooms <= 12 {
            self.phase = GamePhase::EndgameExact;
            let est = (live_mushrooms / 3).max(2);
            self.optimum_time_ms = remaining / est;
            self.max_depth = 64;
            self.use_aspiration = true;
        } else if remaining > Self::CONSERVE_MS {
            self.phase = GamePhase::MidgameFull;
            let est = (live_mushrooms / 4).max(4);
            self.optimum_time_ms = remaining / est;
            self.max_depth = 20;
            self.use_aspiration = true;
        } else if remaining > Self::EMERGENCY_MS {
            self.phase = GamePhase::MidgameConserve;
            let est = (live_mushrooms / 4).max(3);
            self.optimum_time_ms = (remaining / est).min(80);
            self.max_depth = 14;
            self.use_aspiration = true;
        } else {
            self.phase = GamePhase::Emergency;
            self.optimum_time_ms = remaining;
            self.max_depth = 2;
            self.use_aspiration = false;
        }
    }
}
