mod bots;
mod arena;
mod data;

use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;
use rayon::prelude::*;
use crate::arena::play_match;
use crate::bots::bot_list;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut games_per_pair = 100u32;
    let mut seed = 42u64;
    let mut budget_ms = 50u64;
    let mut out_file = String::new();
    let mut progress = false;
    let mut focus: Option<usize> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--games" => { i += 1; games_per_pair = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(100); }
            "--seed" => { i += 1; seed = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(42); }
            "--budget" => { i += 1; budget_ms = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(50); }
            "--out" => { i += 1; out_file = args.get(i).cloned().unwrap_or_default(); }
            "--progress" => { progress = true; }
            "--focus" => { i += 1; focus = args.get(i).and_then(|s| s.parse().ok()); }
            _ => {}
        }
        i += 1;
    }

    let bots = bot_list();
    let n_bots = bots.len();

    let pairs: Vec<(usize, usize)> = if let Some(f) = focus {
        if f >= n_bots {
            eprintln!("focus bot index {} out of range (0-{})", f, n_bots - 1);
            return;
        }
        let others: Vec<usize> = (0..n_bots).filter(|&x| x != f).collect();
        let mut p = Vec::new();
        for &o in &others {
            for _ in 0..games_per_pair {
                p.push((f, o));
                p.push((o, f));
            }
        }
        p
    } else {
        let n_pairs = n_bots * (n_bots - 1);
        (0..n_bots)
            .flat_map(|a| (0..n_bots).filter(move |&b| a != b).map(move |b| (a, b)))
            .flat_map(|pair| std::iter::repeat(pair).take(games_per_pair as usize))
            .collect()
    };

    let total_games = pairs.len();
    eprintln!("Trainer: {} bots, {} total games, budget={}ms", n_bots, total_games, budget_ms);

    let all_logs = Mutex::new(Vec::new());
    let start = Instant::now();
    let completed = Mutex::new(0u32);
    let wins_a = Mutex::new(0u32);
    let draws = Mutex::new(0u32);

    pairs.par_iter().enumerate().for_each(|(gi, &(a_idx, b_idx))| {
        let bot_a = bots[a_idx];
        let bot_b = bots[b_idx];
        let game_seed = seed + gi as u64 * 7919;
        let first_goes_first = gi % 2 == 0;
        let (margin, logs) = play_match(
            bot_a, bot_b, game_seed, first_goes_first, budget_ms,
            a_idx as u32, b_idx as u32, gi as u32,
        );
        {
            let mut c = completed.lock().unwrap();
            *c += 1;
            if margin > 0 { *wins_a.lock().unwrap() += 1; }
            else if margin == 0 { *draws.lock().unwrap() += 1; }
            all_logs.lock().unwrap().extend(logs);

            if progress && *c % (total_games as u32 / 100).max(1) == 0 {
                let w = *wins_a.lock().unwrap();
                let d = *draws.lock().unwrap();
                print_progress(*c, total_games as u32, w, d, start.elapsed().as_secs_f64());
            }
        }
    });

    let logs = all_logs.into_inner().unwrap();
    let w = *wins_a.lock().unwrap();
    let d = *draws.lock().unwrap();
    let elapsed = start.elapsed().as_secs_f64();
    eprintln!("\nDone: {} games in {:.1}s ({:.0} g/s)", total_games, elapsed, total_games as f64 / elapsed.max(0.001));
    eprintln!("A wins={w}, draws={d}, B wins={}", total_games as u32 - w - d);

    // Per-opponent stats
    if focus.is_some() {
        if out_file.is_empty() {
            eprintln!("Stats per opponent:");
            for (idx, (name, _)) in bots.iter().enumerate() {
                if Some(idx) == focus { continue; }
                let opp_logs: Vec<_> = logs.iter().filter(|l| l.bot_b == idx as u32).collect();
                let opp_logs2: Vec<_> = logs.iter().filter(|l| l.bot_a == idx as u32).collect();
                if !opp_logs.is_empty() || !opp_logs2.is_empty() {
                    let a_wins = opp_logs.iter().filter(|l| l.outcome > 0.5).count();
                    let b_wins = opp_logs2.iter().filter(|l| l.outcome < 0.5).count();
                    let draws_c = opp_logs.iter().filter(|l| l.outcome == 0.5).count()
                        + opp_logs2.iter().filter(|l| l.outcome == 0.5).count();
                    let total = opp_logs.len() + opp_logs2.len();
                    if total > 0 {
                        let win_pct = (a_wins + b_wins) as f64 / total as f64 * 100.0;
                        eprintln!("  vs {name:>14}: {a_wins}+{b_wins}W/{draws_c}D/{}L ({win_pct:.0}%)",
                            total - a_wins - b_wins - draws_c);
                    }
                }
            }
        }
    }

    if !out_file.is_empty() {
        if let Err(e) = data::write_logs_csv(&out_file, &logs) {
            eprintln!("Failed to write {}: {}", out_file, e);
        } else {
            eprintln!("Wrote {} log entries to {}", logs.len(), out_file);
        }
    }
}

fn print_progress(done: u32, total: u32, wins: u32, draws: u32, elapsed_s: f64) {
    let total = total.max(1);
    let width = 28usize;
    let filled = ((done as f64 / total as f64) * width as f64).round() as usize;
    let filled = filled.min(width);
    let bar = format!(
        "{}{}",
        "#".repeat(filled),
        "-".repeat(width.saturating_sub(filled))
    );
    let losses = done.saturating_sub(wins + draws);
    let eta = if done > 0 {
        (elapsed_s / done as f64) * (total - done) as f64
    } else {
        0.0
    };
    eprint!(
        "\r[{}] {}/{}  W:{} D:{} L:{}  {:.1}s eta:{:>6.1}s",
        bar,
        done,
        total,
        wins,
        draws,
        losses,
        elapsed_s,
        eta
    );
    let _ = std::io::stderr().flush();
    if done == total {
        eprintln!();
    }
}
