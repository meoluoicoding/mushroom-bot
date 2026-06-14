use std::fs::File;
use std::io::{self, Write};

use mushroom_bot::board::Board;
use mushroom_bot::dataloader::{load_data_bin, read_weights_from_txt, EvalWeights, GameData};
use mushroom_bot::movegen::generate_rectangles;
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::side::GameSide;
use mushroom_bot::timeman::TimeManager;
use mushroom_bot::types::*;

#[derive(Clone, Copy, Debug)]
struct SampleResult {
    live_count: u32,
    legal_moves: usize,
    bot_move: Move,
    bot_score: i32,
    best_score: i32,
    best_rank: usize,
    passed: bool,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut count = 100u32;
    let mut seed = 42u64;
    let mut budget_ms = 50u64;
    let mut dump_init_path: Option<String> = None;
    let mut print_boards = false;
    let mut i = 1usize;

    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                i += 1;
                count = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(100);
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(42);
            }
            "--budget" => {
                i += 1;
                budget_ms = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(50);
            }
            "--dump-init" => {
                i += 1;
                dump_init_path = args.get(i).cloned();
            }
            "--print-boards" => {
                print_boards = true;
            }
            _ => {}
        }
        i += 1;
    }

    let (weights, game_data) = load_weights_and_data();
    let mut dump_file = if let Some(path) = dump_init_path {
        Some(File::create(path)?)
    } else {
        None
    };

    let mut results = Vec::with_capacity(count as usize);
    for idx in 0..count {
        let board_seed = seed + idx as u64 * 7919;
        let board = generate_game_like_board(board_seed);
        let init_line = board_to_init_line(&board);

        if let Some(file) = dump_file.as_mut() {
            writeln!(file, "{}", init_line)?;
        }
        if print_boards {
            println!("BOARD {}", idx + 1);
            println!("{}", init_line);
        }

        let result = test_opening(&board, budget_ms, weights, game_data.clone(), board_seed);
        results.push(result);

        println!(
            "#{:03} seed={} live={} legal={} move={:?} rank={} bot={} best={}{}",
            idx + 1,
            board_seed,
            result.live_count,
            result.legal_moves,
            result.bot_move,
            result.best_rank + 1,
            result.bot_score,
            result.best_score,
            if result.passed { " PASS" } else { "" }
        );
    }

    print_summary(&results);
    Ok(())
}

fn load_weights_and_data() -> (EvalWeights, Option<GameData>) {
    for path in &["data/data.bin", "data.bin"] {
        if let Some(mut gd) = load_data_bin(path) {
            if gd.mquality.is_none() {
                for mq_path in &["data/mquality.bin", "mquality.bin"] {
                    if let Some(mq) = mushroom_bot::mquality::load_mquality_bin(mq_path) {
                        gd.mquality = Some(mq);
                        break;
                    }
                }
            }
            return (gd.weights, Some(gd));
        }
    }

    for path in &["balanced.txt", "weights.txt", "attacker.txt", "defender.txt"] {
        if let Some(w) = read_weights_from_txt(path) {
            return (w, None);
        }
    }

    (EvalWeights::default(), Some(GameData::default()))
}

fn make_search(board: &Board, budget_ms: u64, weights: EvalWeights, game_data: Option<GameData>) -> Search {
    let base_config = SearchConfig {
        time_budget_ms: budget_ms.max(1),
        use_tt: true,
        use_ordering: true,
        use_second_bonus: true,
        use_aspiration: true,
        use_mcts: false,
        use_qsearch: true,
        use_lmr: true,
        use_futility: true,
        use_mquality: true,
        use_exact_endgame: true,
        use_nmp: false,
        use_singular_extension: false,
        use_mtd: false,
    };

    let mut search = if let Some(gd) = game_data {
        Search::with_game_data(base_config, gd)
    } else {
        Search::with_weights(base_config, weights)
    };

    let side = GameSide::from_player(board.player);
    let phase = TimeManager::new().phase(board.live_mask.popcount(), generate_rectangles(&board.values).len());
    search.config = side.tuning().search_config(search.config, phase);
    search.set_side(side);
    search
}

fn test_opening(
    board: &Board,
    budget_ms: u64,
    weights: EvalWeights,
    game_data: Option<GameData>,
    _seed: u64,
) -> SampleResult {
    let rects = generate_rectangles(&board.values);
    let live_count = board.live_mask.popcount();

    if rects.is_empty() {
        return SampleResult {
            live_count,
            legal_moves: 0,
            bot_move: PASS,
            bot_score: 0,
            best_score: 0,
            best_rank: 0,
            passed: true,
        };
    }

    let mut search = make_search(board, budget_ms, weights, game_data);
    let mv = search.think(board).action;
    let bot_move = if board.is_legal_action(mv) { mv } else { PASS };
    let bot_score = score_move(board, bot_move);

    let mut scored: Vec<(i32, Move)> = rects
        .iter()
        .map(|r| {
            let mv = r.to_move();
            (score_move(board, mv), mv)
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let best_score = scored.first().map(|s| s.0).unwrap_or(0);
    let best_rank = scored
        .iter()
        .position(|&(_, m)| m == bot_move)
        .unwrap_or(scored.len().saturating_sub(1));

    SampleResult {
        live_count,
        legal_moves: rects.len(),
        bot_move,
        bot_score,
        best_score,
        best_rank,
        passed: bot_move == PASS,
    }
}

fn score_move(board: &Board, mv: Move) -> i32 {
    if mv == PASS {
        return -10_000;
    }
    board.action_score(mv).0
}

fn generate_game_like_board(seed: u64) -> Board {
    for attempt in 0..24u64 {
        let mut rng = Lcg::new(seed ^ 0x9e3779b97f4a7c15 ^ (attempt.wrapping_mul(0x5851_f42d_4c95_7f2d)));
        let mut values = [4u8; N_CELLS];

        let hotspots = build_hotspots(&mut rng);
        let cold_spots = build_coldspots(&mut rng);
        let row_wave = (rng.next_u32() % 5) as i32;
        let col_wave = (rng.next_u32() % 5) as i32;

        for r in 0..ROWS {
            for c in 0..COLS {
                let idx = cell_index(r, c);
                let mut score = 4i32;

                // Baseline map texture: slightly richer edges, but keep a softer center.
                let edge_dist = r.min(ROWS - 1 - r).min(c.min(COLS - 1 - c)) as i32;
                let center_r = (ROWS as i32 - 1) / 2;
                let center_c = (COLS as i32 - 1) / 2;
                let dr = (r as i32 - center_r).abs();
                let dc = (c as i32 - center_c).abs();
                score += (4 - edge_dist).max(0) / 2;
                score += (4 - (dr + dc) / 4).max(0);

                score += row_bias(r, row_wave);
                score += col_bias(c, col_wave);
                score += hotspot_score(r, c, &hotspots);

                // Cold spots carve out playable corridors / holes like the real map.
                score -= coldspot_score(r, c, &cold_spots);

                // Small local roughness.
                let jitter = (rng.next_u32() % 5) as i32 - 2;
                score += jitter / 2;

                values[idx] = score.clamp(1, 9) as u8;
            }
        }

        add_edge_clusters(&mut values, seed ^ attempt);
        ensure_opening_lane(&mut values, seed ^ attempt);

        let board = Board::from_parts(values, [0i8; N_CELLS], FIRST, 0);
        let legal = generate_rectangles(&board.values).len();
        if (20..=90).contains(&legal) {
            return board;
        }
    }

    // Fallback if the randomized filters failed to find a nicely shaped board.
    let mut rng = Lcg::new(seed ^ 0x9e3779b97f4a7c15);
    let mut values = [4u8; N_CELLS];
    for r in 0..ROWS {
        for c in 0..COLS {
            let idx = cell_index(r, c);
            values[idx] = (3 + ((r + c) % 4) as u8).clamp(1, 9);
        }
    }
    let _ = &mut rng;
    ensure_opening_lane(&mut values, seed);
    Board::from_parts(values, [0i8; N_CELLS], FIRST, 0)
}

fn build_hotspots(rng: &mut Lcg) -> Vec<(i32, i32, i32)> {
    let mut hotspots = Vec::new();
    let anchors = [
        (0i32, 0i32),
        (0, (COLS - 1) as i32),
        ((ROWS - 1) as i32, 0),
        ((ROWS - 1) as i32, (COLS - 1) as i32),
        ((ROWS / 2) as i32, (COLS / 2) as i32),
    ];
    for &(ar, ac) in &anchors {
        let r = (ar + (rng.next_u32() % 3) as i32 - 1).clamp(0, (ROWS - 1) as i32);
        let c = (ac + (rng.next_u32() % 5) as i32 - 2).clamp(0, (COLS - 1) as i32);
        let amp = 3 + (rng.next_u32() % 5) as i32;
        hotspots.push((r, c, amp));
    }
    hotspots
}

fn build_coldspots(rng: &mut Lcg) -> Vec<(i32, i32, i32)> {
    let mut coldspots = Vec::new();
    for _ in 0..3 {
        let r = (rng.next_u32() % ROWS as u32) as i32;
        let c = (rng.next_u32() % COLS as u32) as i32;
        let amp = 2 + (rng.next_u32() % 3) as i32;
        coldspots.push((r, c, amp));
    }
    coldspots
}

fn hotspot_score(r: usize, c: usize, hotspots: &[(i32, i32, i32)]) -> i32 {
    let mut score = 0i32;
    for &(hr, hc, amp) in hotspots {
        let dr = (r as i32 - hr).abs();
        let dc = (c as i32 - hc).abs();
        let dist = dr + dc;
        score += (amp - dist / 2).max(0);
    }
    score
}

fn coldspot_score(r: usize, c: usize, coldspots: &[(i32, i32, i32)]) -> i32 {
    let mut score = 0i32;
    for &(hr, hc, amp) in coldspots {
        let dr = (r as i32 - hr).abs();
        let dc = (c as i32 - hc).abs();
        let dist = dr + dc;
        score += (amp - dist / 2).max(0);
    }
    score
}

fn row_bias(r: usize, wave: i32) -> i32 {
    let center = (ROWS as i32 - 1) / 2;
    let dist = (r as i32 - center).abs();
    ((4 - dist).max(0) + wave) / 2
}

fn col_bias(c: usize, wave: i32) -> i32 {
    let center = (COLS as i32 - 1) / 2;
    let dist = (c as i32 - center).abs();
    ((5 - dist).max(0) + wave) / 2
}

fn ensure_opening_lane(values: &mut [u8; N_CELLS], seed: u64) {
    let row = (seed as usize) % ROWS;
    let start_col = ((seed >> 8) as usize) % (COLS - 9);
    for offset in 0..10 {
        let idx = cell_index(row, start_col + offset);
        values[idx] = 1;
    }

    // Add a small 2x5 lane too so the board has more than one obvious opening.
    let row2 = (row + 3) % ROWS;
    let start_col2 = ((seed >> 16) as usize) % (COLS - 4);
    for dr in 0..2 {
        for dc in 0..5 {
            let idx = cell_index((row2 + dr) % ROWS, start_col2 + dc);
            values[idx] = 1;
        }
    }
}

fn add_edge_clusters(values: &mut [u8; N_CELLS], seed: u64) {
    let mut rng = Lcg::new(seed ^ 0x5bf0_3635_d3f9_0f1d);
    let corners = [
        (0usize, 0usize),
        (0usize, COLS - 4),
        (ROWS - 2, 0usize),
        (ROWS - 2, COLS - 4),
    ];

    for &(r0, c0) in &corners {
        let shift_r = (rng.next_u32() % 2) as usize;
        let shift_c = (rng.next_u32() % 3) as usize;
        let r_base = (r0 + shift_r).min(ROWS - 2);
        let c_base = (c0 + shift_c).min(COLS - 4);
        for dr in 0..2 {
            for dc in 0..4 {
                let idx = cell_index(r_base + dr, c_base + dc);
                values[idx] = values[idx].saturating_add(3).min(9);
            }
        }
    }

    // Add one or two central edge bands like the real maps.
    let band_row = (rng.next_u32() as usize % ROWS).clamp(1, ROWS - 2);
    for c in 0..COLS {
        let idx = cell_index(band_row, c);
        if c % 5 == 0 || c % 5 == 1 {
            values[idx] = values[idx].saturating_add(2).min(9);
        }
    }
}

fn board_to_init_line(board: &Board) -> String {
    let mut rows = Vec::with_capacity(ROWS);
    for r in 0..ROWS {
        let mut row = String::with_capacity(COLS);
        for c in 0..COLS {
            let idx = cell_index(r, c);
            row.push(char::from(b'0' + board.values[idx]));
        }
        rows.push(row);
    }
    format!("INIT {}", rows.join(" "))
}

fn print_summary(results: &[SampleResult]) {
    if results.is_empty() {
        println!("SUMMARY count=0");
        return;
    }

    let count = results.len() as f64;
    let top1 = results.iter().filter(|r| r.best_rank == 0).count();
    let top3 = results.iter().filter(|r| r.best_rank < 3).count();
    let passes = results.iter().filter(|r| r.passed).count();
    let avg_rank = results.iter().map(|r| (r.best_rank + 1) as f64).sum::<f64>() / count;
    let avg_gap = results.iter().map(|r| (r.best_score - r.bot_score) as f64).sum::<f64>() / count;
    let avg_live = results.iter().map(|r| r.live_count as f64).sum::<f64>() / count;
    let avg_moves = results.iter().map(|r| r.legal_moves as f64).sum::<f64>() / count;

    println!();
    println!(
        "SUMMARY samples={} top1={} top3={} pass={} avg_rank={:.2} avg_gap={:.2} avg_live={:.1} avg_legal={:.1}",
        results.len(),
        top1,
        top3,
        passes,
        avg_rank,
        avg_gap,
        avg_live,
        avg_moves
    );
    println!(
        "TOP1_RATE={:.1}% TOP3_RATE={:.1}%",
        top1 as f64 * 100.0 / count,
        top3 as f64 * 100.0 / count
    );
}

struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }
}
