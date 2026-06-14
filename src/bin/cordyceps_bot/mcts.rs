use std::time::Instant;
use mushroom_bot::board::Board;
use mushroom_bot::types::*;
use crate::eval::lightweight_evaluate;
use crate::movegen::generate_moves;

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn usize_range(&mut self, upper: usize) -> usize {
        if upper <= 1 { 0 } else { (self.next() as usize) % upper }
    }
}

pub fn mcts_search(board: &Board, budget_ms: u64) -> Option<(i8, i8, i8, i8)> {
    let moves = generate_moves(board);
    if moves.is_empty() { return None; }
    if moves.len() == 1 { return Some(moves[0]); }

    let start = Instant::now();
    let n = moves.len();
    let mut visits = vec![0u32; n];
    let mut totals = vec![0.0f32; n];
    let mut rng = SimpleRng::new(0x9e3779b97f4a7c15);
    let mut iter = 0u32;
    let budget = budget_ms;

    while (start.elapsed().as_millis() as u64) < budget && iter < 256 {
        iter += 1;
        let idx = select(&visits, &totals, iter, &mut rng);
        let child = board.apply_action(moves[idx]);
        let reward = rollout(&child, board.player, 6, &mut rng);
        visits[idx] += 1;
        totals[idx] += reward;
    }

    let mut best = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for i in 0..n {
        if visits[i] > 0 {
            let avg = totals[i] / visits[i] as f32;
            if avg > best_score { best_score = avg; best = i; }
        }
    }
    if visits[best] > 0 { Some(moves[best]) } else { Some(moves[0]) }
}

fn select(visits: &[u32], totals: &[f32], _iter: u32, rng: &mut SimpleRng) -> usize {
    let total_visits: u32 = visits.iter().sum();
    if total_visits == 0 { return rng.usize_range(visits.len()); }

    let mut best = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    let exploration = 1.414f32;

    for i in 0..visits.len() {
        if visits[i] == 0 { return i; }
        let exploit = totals[i] / visits[i] as f32;
        let explore = exploration * ((total_visits as f32).ln() / visits[i] as f32).sqrt();
        let score = exploit + explore;
        if score > best_score { best_score = score; best = i; }
    }
    best
}

fn rollout(board: &Board, root_player: i8, max_depth: i32, rng: &mut SimpleRng) -> f32 {
    let mut current = board.clone();
    for _ in 0..max_depth {
        if current.is_terminal() {
            let val = current.terminal_score();
            return val * if root_player == current.player { 1.0 } else { -1.0 };
        }
        let moves = generate_moves(&current);
        if moves.is_empty() {
            current = current.apply_action(PASS);
        } else {
            let idx = rng.usize_range(moves.len());
            current = current.apply_action(moves[idx]);
        }
    }
    let val = lightweight_evaluate(&current.values, &current.owners, current.player) as f32;
    val * if root_player == current.player { 1.0 } else { -1.0 }
}
