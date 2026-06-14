use std::process::Command;
use std::time::Instant;

struct Candidate {
    depth: u8,
    budget: u64,
    fitness: f64,
}

fn evaluate_candidate(cand: &Candidate, baseline_depth: u8, _baseline_budget: u64, games: u32, swap: bool) -> f64 {
    let mut cmd = Command::new(".\\target\\release\\tournament.exe");
    cmd.args([
        "--games", &games.to_string(),
        "--depth-a", &cand.depth.to_string(),
        "--budget", &cand.budget.to_string(),
        "--depth-b", &baseline_depth.to_string(),
        "--seed", "42",
    ]);
    if swap { cmd.arg("--swap"); }

    let output = cmd.output().expect("tournament failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
    if parts.len() >= 4 {
        parts[3].parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    }
}

fn mutate(depth: u8, budget: u64, rng: &mut fastrand::Rng) -> (u8, u64) {
    let new_depth = match rng.u8(0, 2) {
        0 => depth.saturating_sub(1).max(1),
        1 => depth,
        _ => (depth + 1).min(6),
    };
    let new_budget = match rng.u8(0, 2) {
        0 => (budget / 2).max(5),
        1 => budget,
        _ => (budget * 2).min(200),
    };
    (new_depth, new_budget)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut generations = 10u32;
    let mut population = 8u32;
    let mut games = 6u32;
    let mut baseline_depth = 3u8;
    let mut baseline_budget = 30u64;
    let mut swap = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--gen" | "--generations" => { i += 1; generations = args[i].parse().unwrap_or(10); }
            "--pop" | "--population" => { i += 1; population = args[i].parse().unwrap_or(8); }
            "--games" => { i += 1; games = args[i].parse().unwrap_or(6); }
            "--baseline-depth" => { i += 1; baseline_depth = args[i].parse().unwrap_or(3); }
            "--baseline-budget" => { i += 1; baseline_budget = args[i].parse().unwrap_or(30); }
            "--no-swap" => { swap = false; }
            _ => {}
        }
        i += 1;
    }

    eprintln!("Tuner: {generations} gen, population {population}, {games} games/eval");
    eprintln!("Baseline: depth={baseline_depth}, budget={baseline_budget}ms");

    let mut rng = fastrand::Rng::with_seed(12345);

    // Initialize population
    let mut pop: Vec<Candidate> = (0..population)
        .map(|_| Candidate {
            depth: rng.u8(1, 4),
            budget: 10 * rng.u8(1, 6) as u64,
            fitness: 0.0,
        })
        .collect();

    // Always include baseline
    pop[0] = Candidate { depth: baseline_depth, budget: baseline_budget, fitness: 0.0 };

    for gen in 0..generations {
        let start = Instant::now();
        eprintln!("\n=== Generation {}/{} ===", gen + 1, generations);

        // Evaluate
        for cand in &mut pop {
            cand.fitness = evaluate_candidate(cand, baseline_depth, baseline_budget, games, swap);
        }

        pop.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        let elapsed = start.elapsed().as_secs();
        eprintln!("Best: depth={}, budget={}ms, fitness={:.2}  ({elapsed}s)",
            pop[0].depth, pop[0].budget, pop[0].fitness);

        // Keep top 25%, replace rest with mutated copies
        let keep = (population / 4).max(1) as usize;
        for j in keep..pop.len() {
            let parent = &pop[rng.u8(0, keep as u8 - 1) as usize];
            let (d, b) = mutate(parent.depth, parent.budget, &mut rng);
            pop[j] = Candidate { depth: d, budget: b, fitness: 0.0 };
        }
    }

    // Final result
    eprintln!("\n=== Best config ===");
    eprintln!("depth={}, budget={}ms, fitness={:.2}", pop[0].depth, pop[0].budget, pop[0].fitness);
    println!("{} {} {:.4}", pop[0].depth, pop[0].budget, pop[0].fitness);
}

mod fastrand {
    pub struct Rng(u64);
    impl Rng {
        pub fn with_seed(seed: u64) -> Self { Self(seed) }
        pub fn u8(&mut self, lo: u8, hi: u8) -> u8 {
            if lo > hi { return lo; }
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            lo + (self.0 >> 33) as u8 % (hi - lo + 1)
        }
    }
}
