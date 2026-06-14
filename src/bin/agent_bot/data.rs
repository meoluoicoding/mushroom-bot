use mushroom_bot::dataloader::{load_data_bin, EvalWeights, GameData};

pub struct AgentData {
    pub weights: EvalWeights,
    pub game_data: Option<GameData>,
}

impl AgentData {
    pub fn load() -> Self {
        for path in &["data/data.bin", "data.bin"] {
            if let Some(gd) = load_data_bin(path) {
                return Self { weights: gd.weights, game_data: Some(gd) };
            }
        }
        Self {
            weights: EvalWeights {
                territory: 100.0, safe_territory: 200.0, vulnerability: -15.0,
                steal_potential: 50.0, mobility: 8.0, connectivity: 12.0,
                corner_bonus: 25.0, edge_bonus: 5.0,
            },
            game_data: Some(GameData::default()),
        }
    }
}
