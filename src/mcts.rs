use std::time::Instant;

use crate::board::Board;
use crate::dataloader::{EvalWeights, GameData};
use crate::movegen::{generate_rectangles, RectInfo};
use crate::types::*;

pub struct MctsResult {
    pub action: Move,
    pub value: f32,
    pub visits: u32,
}

pub fn root_mcts_search(
    board: &Board,
    candidates: &[Move],
    weights: &EvalWeights,
    game_data: Option<&GameData>,
    budget_ms: u64,
) -> Option<MctsResult> {
    if candidates.is_empty() {
        return None;
    }

    if candidates.len() == 1 {
        return Some(MctsResult {
            action: candidates[0],
            value: perspective_eval(&board.apply_action(candidates[0]), board.player, weights, game_data),
            visits: 1,
        });
    }

    let mut visits = vec![0u32; candidates.len()];
    let mut totals = vec![0.0f32; candidates.len()];
    let start = Instant::now();
    let root_player = board.player;
    let mut rng = Rng::new(0x9e3779b97f4a7c15);
    let mut iter = 0u32;
    let exploration_scale = weights.territory.max(1.0);

    while (start.elapsed().as_millis() as u64) < budget_ms && iter < 256 {
        iter += 1;
        let idx = select_candidate(&visits, &totals, iter, exploration_scale, &mut rng);
        let child = board.apply_action(candidates[idx]);
        let reward = rollout(
            &child,
            root_player,
            weights,
            game_data,
            6,
            &mut rng,
        );
        visits[idx] += 1;
        totals[idx] += reward;
    }

    let mut best_idx = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for i in 0..candidates.len() {
        if visits[i] == 0 {
            continue;
        }
        let avg = totals[i] / visits[i] as f32;
        if avg > best_score {
            best_score = avg;
            best_idx = i;
        }
    }

    Some(MctsResult {
        action: candidates[best_idx],
        value: if visits[best_idx] == 0 {
            0.0
        } else {
            totals[best_idx] / visits[best_idx] as f32
        },
        visits: visits[best_idx],
    })
}

fn select_candidate(visits: &[u32], totals: &[f32], iter: u32, exploration_scale: f32, rng: &mut Rng) -> usize {
    for (i, &v) in visits.iter().enumerate() {
        if v == 0 {
            return i;
        }
    }

    let total_visits = iter.max(1) as f32;
    let mut best_idx = 0usize;
    let mut best_ucb = f32::NEG_INFINITY;
    for i in 0..visits.len() {
        let avg = totals[i] / visits[i] as f32;
        let exploration = (2.0 * total_visits.ln() / visits[i] as f32).sqrt();
        let jitter = (rng.next_u32() as f32 / u32::MAX as f32) * 0.001;
        let ucb = avg + 1.4 * exploration_scale * exploration + jitter;
        if ucb > best_ucb {
            best_ucb = ucb;
            best_idx = i;
        }
    }
    best_idx
}

fn rollout(
    board: &Board,
    root_player: i8,
    weights: &EvalWeights,
    game_data: Option<&GameData>,
    depth: u8,
    rng: &mut Rng,
) -> f32 {
    let mut current = board.clone();
    let mut remaining = depth;

    while remaining > 0 && !current.is_terminal() {
        let rects = generate_rectangles(&current.values);
        if rects.is_empty() {
            current = current.apply_action(PASS);
            if current.is_terminal() {
                break;
            }
            remaining -= 1;
            continue;
        }

        let mv = choose_rollout_move(&current, &rects, rng);
        current = current.apply_action(mv);
        remaining -= 1;
    }

    perspective_eval(&current, root_player, weights, game_data)
}

fn choose_rollout_move(board: &Board, rects: &[RectInfo], rng: &mut Rng) -> Move {
    let mut scored: Vec<(i32, Move)> = rects
        .iter()
        .map(|r| {
            let mv = r.to_move();
            let (sd, rec, fresh, _live, _, area) = board.action_score(mv);
            let score = sd + rec * 25 + fresh * 5 + area * 3;
            (score, mv)
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let top_n = scored.len().min(4);
    let idx = rng.gen_range(top_n);
    scored[idx].1
}

fn perspective_eval(board: &Board, root_player: i8, weights: &EvalWeights, game_data: Option<&GameData>) -> f32 {
    let value = board.evaluate(weights, game_data);
    if board.player == root_player {
        value
    } else {
        -value
    }
}

struct Rng(u64);

impl Rng {
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

    fn gen_range(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            return 0;
        }
        (self.next_u32() as usize) % upper
    }
}
