use mushroom_bot::types::*;

pub struct RegressionPosition {
    pub name: &'static str,
    pub values: [u8; N_CELLS],
    pub owners: [i8; N_CELLS],
    pub player: i8,
    pub expected_move: Move,
}

pub fn get_regression_positions() -> Vec<RegressionPosition> {
    let mut positions = Vec::new();

    // Position 1: Only one option (a 1x1 rect at 0,0 containing 10)
    let mut values1 = [0u8; N_CELLS];
    values1[cell_index(0, 0)] = 10;
    positions.push(RegressionPosition {
        name: "Single mushroom top-left",
        values: values1,
        owners: [0i8; N_CELLS],
        player: FIRST,
        expected_move: (0, 0, 0, 0),
    });

    // Position 2: Capture fight, choosing the higher-value option
    // (0,0) is 10. (1,0) is 9. First player should choose (0,0) over (1,0).
    let mut values2 = [0u8; N_CELLS];
    values2[cell_index(0, 0)] = 10;
    values2[cell_index(1, 0)] = 9;
    positions.push(RegressionPosition {
        name: "Choose higher value mushroom",
        values: values2,
        owners: [0i8; N_CELLS],
        player: FIRST,
        expected_move: (0, 0, 0, 0),
    });

    positions
}
