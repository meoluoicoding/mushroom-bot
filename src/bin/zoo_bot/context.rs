use mushroom_bot::dataloader::{load_data_bin, EvalWeights, GameData};

#[derive(Clone)]
pub struct EvalContext {
    pub weights: EvalWeights,
    pub game_data: Option<GameData>,
}

impl EvalContext {
    pub fn load() -> Self {
        for path in ["data/data.bin", "data.bin"] {
            if let Some(mut gd) = load_data_bin(path) {
                if gd.mquality.is_none() {
                    for mq_path in ["data/mquality.bin", "mquality.bin"] {
                        if let Some(mq) = mushroom_bot::mquality::load_mquality_bin(mq_path) {
                            gd.mquality = Some(mq);
                            break;
                        }
                    }
                }
                return Self {
                    weights: gd.weights,
                    game_data: Some(gd),
                };
            }
        }

        Self {
            weights: EvalWeights::default(),
            game_data: Some(GameData::default()),
        }
    }
}
