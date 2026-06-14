use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::time::Instant;

use rayon::prelude::*;

use mushroom_bot::board::Board;
use mushroom_bot::dataloader::{load_data_bin, read_weights_from_txt, EvalWeights, GameData};
use mushroom_bot::eval::{compute_move_features, static_move_score};
use mushroom_bot::movegen::{generate_rectangles, RectInfo};
use mushroom_bot::mquality::MoveQualityTable;
use mushroom_bot::types::*;

#[derive(Clone, Copy)]
struct Choice {
    action: Move,
    rect_id: u16,
}

#[derive(Clone)]
struct MoveLog {
    game_id: u32,
    ply: u32,
    mover: i8,
    rect_id: u16,
    phase: usize,
    bucket: usize,
    move_value: f32,
    outcome: f32,
    margin: i32,
}

#[derive(Clone)]
struct EvalContext {
    weights: EvalWeights,
    game_data: Option<GameData>,
}

impl EvalContext {
    fn load(data_path: &str, weights_path: Option<&str>) -> Self {
        let mut weights = EvalWeights::default();
        let mut game_data = None;

        if let Some(gd) = load_data_bin(data_path) {
            weights = gd.weights;
            game_data = Some(gd);
        }

        if let Some(path) = weights_path {
            if let Some(w) = read_weights_from_txt(path) {
                weights = w;
            }
        }

        Self { weights, game_data }
    }
}

fn random_board(seed: u64) -> Board {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut values = [0u8; N_CELLS];
    for value in &mut values {
        *value = rng.u8(1, 9);
    }
    Board::from_parts(values, [0i8; N_CELLS], FIRST, 0)
}

fn evaluate(board: &Board, ctx: &EvalContext) -> f32 {
    board.evaluate(&ctx.weights, ctx.game_data.as_ref())
}

fn order_actions(board: &Board, rects: &[RectInfo], ctx: &EvalContext) -> Vec<(i32, Choice)> {
    let phase = MoveQualityTable::phase_for_position(board.live_mask.popcount(), rects.len());
    let mut scored: Vec<(i32, Choice)> = rects
        .iter()
        .map(|rect| {
            let mv = rect.to_move();
            let features = compute_move_features(board, mv);
            let score = static_move_score(&features);
            let mquality_bonus = ctx
                .game_data
                .as_ref()
                .and_then(|gd| gd.mquality.as_ref())
                .map(|mq| {
                    let bucket = MoveQualityTable::score_bucket(score);
                    mq.bonus(rect.id as usize, phase, bucket) as i32
                })
                .unwrap_or(0);
            let total = score + mquality_bonus;
            (
                total,
                Choice {
                    action: mv,
                    rect_id: rect.id,
                },
            )
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored
}

fn negamax(
    board: &Board,
    depth: u8,
    mut alpha: f32,
    beta: f32,
    start: Instant,
    budget_ms: u64,
    ctx: &EvalContext,
) -> f32 {
    if depth == 0 || board.is_terminal() || (start.elapsed().as_millis() as u64) >= budget_ms {
        return evaluate(board, ctx);
    }

    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        let child = board.apply_action(PASS);
        if child.is_terminal() {
            return evaluate(board, ctx);
        }
        return -negamax(&child, depth - 1, -beta, -alpha, start, budget_ms, ctx);
    }

    for (_, choice) in order_actions(board, &rects, ctx) {
        let child = board.apply_action(choice.action);
        let score = -negamax(&child, depth - 1, -beta, -alpha, start, budget_ms, ctx);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }

    alpha
}

fn choose_action(board: &Board, budget_ms: u64, max_depth: u8, ctx: &EvalContext) -> Choice {
    let rects = generate_rectangles(&board.values);
    if rects.is_empty() {
        return Choice {
            action: PASS,
            rect_id: u16::MAX,
        };
    }

    let start = Instant::now();
    let ordered = order_actions(board, &rects, ctx);
    let mut best = ordered[0].1;
    let mut best_value = f32::NEG_INFINITY;

    for (_, choice) in ordered {
        if (start.elapsed().as_millis() as u64) >= budget_ms {
            break;
        }
        let child = board.apply_action(choice.action);
        let value = -negamax(
            &child,
            max_depth.saturating_sub(1),
            f32::NEG_INFINITY,
            f32::INFINITY,
            start,
            budget_ms,
            ctx,
        );
        if value > best_value {
            best_value = value;
            best = choice;
        }
    }

    best
}

fn play_game(
    game_id: u32,
    board_seed: u64,
    first_goes_first: bool,
    depth_a: u8,
    depth_b: u8,
    budget_ms: u64,
    ctx_a: &EvalContext,
    ctx_b: &EvalContext,
) -> (i32, Vec<MoveLog>) {
    let mut board = random_board(board_seed);
    let mut turn: usize = if first_goes_first { 0 } else { 1 };
    let mut logs = Vec::new();
    let mut ply = 0u32;

    loop {
        if board.is_terminal() {
            break;
        }

        let rects = generate_rectangles(&board.values);
        if rects.is_empty() {
            board = board.apply_action(PASS);
            turn ^= 1;
            ply += 1;
            continue;
        }

        let depth = if turn == 0 { depth_a } else { depth_b };
        let ctx = if turn == 0 { ctx_a } else { ctx_b };
        let eval_before = evaluate(&board, ctx);
        let choice = choose_action(&board, budget_ms, depth, ctx);
        let child = board.apply_action(choice.action);
        let eval_after = evaluate(&child, ctx);
        let move_value = eval_after - eval_before;
        let phase = MoveQualityTable::phase_for_position(board.live_mask.popcount(), rects.len());
        let bucket = if choice.action == PASS {
            0
        } else {
            let features = compute_move_features(&board, choice.action);
            let score = static_move_score(&features);
            MoveQualityTable::score_bucket(score)
        };

        logs.push(MoveLog {
            game_id,
            ply,
            mover: board.player,
            rect_id: choice.rect_id,
            phase,
            bucket,
            move_value,
            outcome: 0.0,
            margin: 0,
        });

        board = child;
        turn ^= 1;
        ply += 1;
    }

    let margin = board.score(FIRST) - board.score(SECOND);
    for log in &mut logs {
        log.margin = margin;
        log.outcome = outcome_for_mover(log.mover, margin);
    }

    (margin, logs)
}

fn outcome_for_mover(mover: i8, margin: i32) -> f32 {
    if margin == 0 {
        return 0.5;
    }
    let first_wins = margin > 0;
    let mover_wins = if mover == FIRST { first_wins } else { !first_wins };
    if mover_wins {
        1.0
    } else {
        0.0
    }
}

fn write_logs(log_file: &str, logs: &[MoveLog]) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(log_file).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_file)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "game_id,ply,mover,rect_id,phase,bucket,move_value,outcome,margin")?;
    for log in logs {
        writeln!(
            writer,
            "{},{},{},{},{},{},{:.4},{:.2},{}",
            log.game_id,
            log.ply,
            log.mover,
            log.rect_id,
            log.phase,
            log.bucket,
            log.move_value,
            log.outcome,
            log.margin
        )?;
    }
    writer.flush()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut games = 100u32;
    let mut depth_a = 3u8;
    let mut depth_b = 3u8;
    let mut budget_ms = 20u64;
    let mut seed = 42u64;
    let mut swap = false;
    let mut log_file = String::new();
    let mut data_path = "data/data.bin".to_string();
    let mut weights_path = None::<String>;
    let mut progress = false;

    let mut data_path_a = None::<String>;
    let mut weights_path_a = None::<String>;
    let mut data_path_b = None::<String>;
    let mut weights_path_b = None::<String>;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--games" => {
                i += 1;
                games = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(100);
            }
            "--depth-a" => {
                i += 1;
                depth_a = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(3);
            }
            "--depth-b" => {
                i += 1;
                depth_b = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(3);
            }
            "--budget" => {
                i += 1;
                budget_ms = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(20);
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(42);
            }
            "--swap" => {
                swap = true;
            }
            "--log-file" => {
                i += 1;
                log_file = args.get(i).cloned().unwrap_or_default();
            }
            "--data" => {
                i += 1;
                data_path = args.get(i).cloned().unwrap_or_else(|| "data/data.bin".to_string());
            }
            "--weights" => {
                i += 1;
                weights_path = args.get(i).cloned();
            }
            "--data-a" => {
                i += 1;
                data_path_a = args.get(i).cloned();
            }
            "--weights-a" => {
                i += 1;
                weights_path_a = args.get(i).cloned();
            }
            "--data-b" => {
                i += 1;
                data_path_b = args.get(i).cloned();
            }
            "--weights-b" => {
                i += 1;
                weights_path_b = args.get(i).cloned();
            }
            "--progress" => {
                progress = true;
            }
            _ => {}
        }
        i += 1;
    }

    let final_data_path_a = data_path_a.unwrap_or_else(|| data_path.clone());
    let final_weights_path_a = weights_path_a.or_else(|| weights_path.clone());
    let final_data_path_b = data_path_b.unwrap_or_else(|| data_path.clone());
    let final_weights_path_b = weights_path_b.or_else(|| weights_path.clone());

    let ctx_a = EvalContext::load(&final_data_path_a, final_weights_path_a.as_deref());
    let ctx_b = EvalContext::load(&final_data_path_b, final_weights_path_b.as_deref());

    eprintln!(
        "Tournament: {games} games, depth A={depth_a} B={depth_b}, budget={budget_ms}ms, swap={swap}"
    );

    let total_margin;
    let wins;
    let draws;
    let all_logs = Mutex::new(Vec::new());
    let start = Instant::now();
    let progress_step = (games / 100).max(1);
    let progress_count = Mutex::new(0u32);
    let progress_wins = Mutex::new(0u32);
    let progress_draws = Mutex::new(0u32);
    let progress_margin = Mutex::new(0i64);

    (0..games).into_par_iter().for_each(|game| {
        let first_goes_first = if swap { game % 2 == 0 } else { true };
        let (margin, logs) = play_game(
            game,
            seed + game as u64 * 1000,
            first_goes_first,
            depth_a,
            depth_b,
            budget_ms,
            &ctx_a,
            &ctx_b,
        );

        {
            let mut pc = progress_count.lock().unwrap();
            *pc += 1;
            if margin > 0 {
                *progress_wins.lock().unwrap() += 1;
            } else if margin == 0 {
                *progress_draws.lock().unwrap() += 1;
            }
            *progress_margin.lock().unwrap() += margin as i64;
            all_logs.lock().unwrap().extend(logs);

            if progress && (game + 1 == games || *pc % progress_step == 0) {
                let w = *progress_wins.lock().unwrap();
                let d = *progress_draws.lock().unwrap();
                let m = *progress_margin.lock().unwrap();
                print_progress(*pc, games, w, d, m, start.elapsed().as_secs_f64());
            }
        }
    });

    total_margin = *progress_margin.lock().unwrap();
    wins = *progress_wins.lock().unwrap();
    draws = *progress_draws.lock().unwrap();
    let all_logs = all_logs.into_inner().unwrap();

    if !log_file.is_empty() {
        if let Err(err) = write_logs(&log_file, &all_logs) {
            eprintln!("Failed to write log file {}: {}", log_file, err);
        }
    }

    let avg = total_margin as f64 / games as f64;
    let losses = games - wins - draws;
    eprintln!(
        "Results: +{wins} ={draws} -{losses}  |  avg margin: {avg:.2}  |  win%: {:.1}%",
        wins as f64 / games as f64 * 100.0
    );
    println!("{wins} {draws} {losses} {avg:.2}");
}

fn print_progress(done: u32, total: u32, wins: u32, draws: u32, margin: i64, elapsed_s: f64) {
    let total = total.max(1);
    let width = 28usize;
    let filled = ((done as f64 / total as f64) * width as f64).round() as usize;
    let filled = filled.min(width);
    let bar = format!(
        "{}{}",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    );
    let avg = margin as f64 / done.max(1) as f64;
    let eta = if done > 0 {
        (elapsed_s / done as f64) * (total - done) as f64
    } else {
        0.0
    };
    eprint!(
        "\r[{}] {}/{}  win:{} draw:{}  avg:{:.2}  eta:{:>6.1}s",
        bar,
        done,
        total,
        wins,
        draws,
        avg,
        eta
    );
    let _ = std::io::stderr().flush();
    if done == total {
        eprintln!();
    }
}

mod fastrand {
    pub struct Rng(u64);

    impl Rng {
        pub fn with_seed(seed: u64) -> Self {
            Self(seed)
        }

        pub fn u8(&mut self, lo: u8, hi: u8) -> u8 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            lo + (self.0 >> 33) as u8 % (hi - lo + 1)
        }
    }
}
