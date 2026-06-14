use std::fs;
use std::path::Path;

use mushroom_bot::dataloader::{read_weights_from_txt, EvalWeights};

#[derive(Default)]
struct Stats {
    moves: u32,
    wins: u32,
    draws: u32,
    losses: u32,
    total_margin: i64,
    total_move_value: f64,
    total_outcome: f64,
    phase_moves: [u32; 3],
    phase_outcomes: [f64; 3],
    phase_move_values: [f64; 3],
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut log_file = String::new();
    let mut base_file = String::new();
    let mut output = "data/weights_next.txt".to_string();
    let mut learning_rate = 1.0f32;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--log-file" => {
                i += 1;
                log_file = args.get(i).cloned().unwrap_or_default();
            }
            "--base" => {
                i += 1;
                base_file = args.get(i).cloned().unwrap_or_default();
            }
            "--output" => {
                i += 1;
                output = args
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "data/weights_next.txt".to_string());
            }
            "--lr" => {
                i += 1;
                learning_rate = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            }
            _ => {}
        }
        i += 1;
    }

    if log_file.is_empty() {
        eprintln!("Usage: update_weights --log-file <selfplay.csv> [--base weights.txt] [--output weights_next.txt]");
        std::process::exit(1);
    }

    let stats = match read_stats(&log_file) {
        Some(s) => s,
        None => {
            eprintln!("No usable log records found in {}", log_file);
            std::process::exit(1);
        }
    };

    let base = if !base_file.is_empty() {
        read_weights_from_txt(&base_file).unwrap_or_default()
    } else {
        EvalWeights::default()
    };

    let proposal = propose_weights(&base, &stats, learning_rate);
    write_weights(&output, &proposal).expect("write proposed weights");

    eprintln!("Wrote proposed weights to {}", output);
    eprintln!(
        "Stats: moves={}, wins={}, draws={}, losses={}, avg_margin={:.2}, avg_move_value={:.2}, win_rate={:.1}%",
        stats.moves,
        stats.wins,
        stats.draws,
        stats.losses,
        stats.total_margin as f64 / stats.games_played().max(1) as f64,
        stats.total_move_value / stats.moves.max(1) as f64,
        stats.win_rate() * 100.0
    );
    eprintln!(
        "Proposal: territory={:.0}, safe={:.0}, vuln={:.0}, steal={:.0}, mobility={:.0}, conn={:.0}, corner={:.0}, edge={:.0}",
        proposal.territory,
        proposal.safe_territory,
        proposal.vulnerability,
        proposal.steal_potential,
        proposal.mobility,
        proposal.connectivity,
        proposal.corner_bonus,
        proposal.edge_bonus
    );
}

fn read_stats(log_path: &str) -> Option<Stats> {
    let mut stats = Stats::default();
    let path = Path::new(log_path);

    let mut files = Vec::new();
    if path.is_dir() {
        for entry in fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            let p = entry.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "csv" || ext == "txt" {
                files.push(p);
            }
        }
    } else {
        files.push(path.to_path_buf());
    }

    for file in files {
        let text = fs::read_to_string(&file).ok()?;
        for line in text.lines() {
            if line.starts_with("game_id,") || line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 9 {
                continue;
            }
            let phase = cols[4].trim().parse::<usize>().ok()?.min(2);
            let move_value = cols[6].trim().parse::<f64>().ok()?;
            let outcome = cols[7].trim().parse::<f64>().ok()?;
            let margin = cols[8].trim().parse::<i32>().ok()?;

            stats.moves += 1;
            stats.total_move_value += move_value;
            stats.total_outcome += outcome;
            stats.total_margin += margin as i64;
            stats.phase_moves[phase] += 1;
            stats.phase_outcomes[phase] += outcome;
            stats.phase_move_values[phase] += move_value;

            if margin > 0 {
                stats.wins += 1;
            } else if margin < 0 {
                stats.losses += 1;
            } else {
                stats.draws += 1;
            }
        }
    }
    if stats.moves == 0 {
        None
    } else {
        Some(stats)
    }
}

fn propose_weights(base: &EvalWeights, stats: &Stats, lr: f32) -> EvalWeights {
    let win_rate = stats.win_rate() as f32;
    let avg_margin = stats.total_margin as f32 / stats.games_played().max(1) as f32;
    let avg_move_value = (stats.total_move_value / stats.moves.max(1) as f64) as f32;
    let performance = (win_rate - 0.5) * 2.0 + (avg_margin / 20.0).clamp(-2.0, 2.0);
    let tempo = (avg_move_value / 100.0).clamp(-2.0, 2.0);

    let phase_early = phase_score(stats, 0);
    let phase_mid = phase_score(stats, 1);
    let phase_late = phase_score(stats, 2);

    let defense_bias = if performance < 0.0 { -performance.abs() } else { performance * 0.25 };

    EvalWeights {
        territory: adjust(base.territory, (performance * 10.0 + phase_mid * 2.0) * lr),
        safe_territory: adjust(base.safe_territory, (performance * 8.0 + phase_late * 4.0) * lr),
        vulnerability: adjust(
            base.vulnerability,
            ((-performance * 6.0) + defense_bias * 8.0 + phase_early * -2.0) * lr,
        ),
        steal_potential: adjust(base.steal_potential, (performance * 5.0 + tempo * 4.0) * lr),
        mobility: adjust(base.mobility, (performance * 4.0 + phase_early * 2.0) * lr),
        connectivity: adjust(base.connectivity, (performance * 3.0 + phase_mid * 1.5) * lr),
        corner_bonus: adjust(base.corner_bonus, (performance * 2.0 + phase_early * 1.0) * lr),
        edge_bonus: adjust(base.edge_bonus, (performance * 2.0 + phase_late * 1.0) * lr),
    }
}

fn phase_score(stats: &Stats, phase: usize) -> f32 {
    if phase >= 3 || stats.phase_moves[phase] == 0 {
        return 0.0;
    }
    let avg_outcome = stats.phase_outcomes[phase] / stats.phase_moves[phase] as f64;
    let avg_value = stats.phase_move_values[phase] / stats.phase_moves[phase] as f64;
    (((avg_outcome - 0.5) * 2.0) as f32 + (avg_value as f32 / 100.0)).clamp(-2.0, 2.0)
}

fn adjust(base: f32, delta: f32) -> f32 {
    (base + delta).round().clamp(-500.0, 500.0)
}

fn write_weights(path: &str, weights: &EvalWeights) -> std::io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let text = format!(
        "{:.0}, {:.0}, {:.0}, {:.0}, {:.0}, {:.0}, {:.0}, {:.0}\n",
        weights.territory,
        weights.safe_territory,
        weights.vulnerability,
        weights.steal_potential,
        weights.mobility,
        weights.connectivity,
        weights.corner_bonus,
        weights.edge_bonus
    );
    fs::write(path, text)
}

impl Stats {
    fn games_played(&self) -> u32 {
        self.wins + self.draws + self.losses
    }

    fn win_rate(&self) -> f64 {
        if self.games_played() == 0 {
            0.0
        } else {
            self.wins as f64 / self.games_played() as f64
        }
    }
}
