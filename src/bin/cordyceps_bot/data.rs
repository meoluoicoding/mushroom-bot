use mushroom_bot::dataloader::{load_data_bin, EvalWeights, GameData};

pub struct CordycepsData {
    pub weights: EvalWeights,
    pub game_data: Option<GameData>,
}

impl CordycepsData {
    pub fn load() -> Self {
        for path in &["data/data.bin", "data.bin"] {
            if let Some(gd) = load_data_bin(path) {
                return Self { weights: gd.weights, game_data: Some(gd) };
            }
        }
        Self {
            weights: EvalWeights::default(),
            game_data: Some(GameData::default()),
        }
    }
}
