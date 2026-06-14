use mushroom_bot::types::*;

pub fn count_cells(values: &[u8; N_CELLS], owners: &[i8; N_CELLS], player: i8) -> i32 {
    owners.iter().filter(|&&o| o == player).count() as i32
}

pub fn lightweight_evaluate(values: &[u8; N_CELLS], owners: &[i8; N_CELLS], player: i8) -> i32 {
    let opp = opponent(player);
    let my_cells = count_cells(values, owners, player);
    let opp_cells = count_cells(values, owners, opp);

    let mut connectivity = 0i32;
    for r in 0..ROWS {
        for c in 0..COLS {
            let idx = cell_index(r, c);
            if owners[idx] != player { continue; }
            if c + 1 < COLS && owners[cell_index(r, c + 1)] == player { connectivity += 1; }
            if r + 1 < ROWS && owners[cell_index(r + 1, c)] == player { connectivity += 1; }
        }
    }

    let mut corners = 0i32;
    let mut edges = 0i32;
    let cpos: [(usize, usize); 4] = [(0, 0), (0, COLS - 1), (ROWS - 1, 0), (ROWS - 1, COLS - 1)];
    for (cr, cc) in cpos {
        let o = owners[cell_index(cr, cc)];
        if o == player { corners += 1; } else if o == opp { corners -= 1; }
    }
    for c in 0..COLS {
        if owners[cell_index(0, c)] == player { edges += 1; }
        else if owners[cell_index(0, c)] == opp { edges -= 1; }
        if owners[cell_index(ROWS - 1, c)] == player { edges += 1; }
        else if owners[cell_index(ROWS - 1, c)] == opp { edges -= 1; }
    }
    for r in 1..ROWS - 1 {
        if owners[cell_index(r, 0)] == player { edges += 1; }
        else if owners[cell_index(r, 0)] == opp { edges -= 1; }
        if owners[cell_index(r, COLS - 1)] == player { edges += 1; }
        else if owners[cell_index(r, COLS - 1)] == opp { edges -= 1; }
    }

    // Cordyceps-style weights (territory, safe_territory, vulnerability, steal_potential, mobility, connectivity, corner_bonus, edge_bonus)
    (my_cells - opp_cells) * 100
        + connectivity * 25
        + corners * 50
        + edges * 15
}
