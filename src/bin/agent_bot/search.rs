use crate::board::AgentBoard;
use crate::data::AgentData;
use mushroom_bot::dataloader::EvalWeights;
use crate::eval::{evaluate, legal_moves_prioritized};
use crate::timer::Timer;
use crate::tt::TranspositionTable;
use crate::zobrist::hash_board;
use mushroom_bot::types::*;
use std::time::Instant;

pub static mut NODES_SEARCHED: u64 = 0;

const INF: i32 = 999999;
const TT_EXACT: u8 = 1;
const TT_LOWER: u8 = 2;
const TT_UPPER: u8 = 3;

struct SearchState {
    tt: TranspositionTable,
    start: Instant,
}

impl SearchState {
    fn new() -> Self {
        Self { tt: TranspositionTable::new(), start: Instant::now() }
    }
}

pub fn search_best_move(board: &AgentBoard, time_budget_ms: i64, data: &AgentData) -> (i8, i8, i8, i8) {
    let timer = Timer::new();
    unsafe { NODES_SEARCHED = 0; }

    let mut work = board.clone();
    let moves = legal_moves_prioritized(&work);
    if moves.is_empty() { return (-1, -1, -1, -1); }

    let mut state = SearchState::new();
    state.start = Instant::now();
    let budget = (time_budget_ms * 80 / 100).max(1);

    if moves.len() <= 5 {
        let mut best_mv = moves[0];
        alpha_beta(&mut work, &mut state, 8, -INF, INF, &timer, budget as u64, &mut best_mv, &data.weights);
        if !timer.timed_out(budget as u64) {
            return (best_mv.1, best_mv.2, best_mv.3, best_mv.4);
        }
    }

    let mut best_move = moves[0];
    let mut prev_score = 0i32;
    let max_depth: i16 = 12;

    for depth in 1i16..=max_depth {
        let depth_timer = Timer::new();
        let (alpha, beta) = if depth == 1 { (-INF, INF) } else { (prev_score - 50, prev_score + 50) };
        let mut depth_best = best_move;
        state.tt.increment_age();

        let score = alpha_beta(&mut work, &mut state, depth, alpha, beta, &timer, budget as u64, &mut depth_best, &data.weights);
        let final_score = if score <= alpha || score >= beta {
            alpha_beta(&mut work, &mut state, depth, -INF, INF, &timer, budget as u64, &mut depth_best, &data.weights)
        } else { score };

        if !timer.timed_out(budget as u64) {
            best_move = depth_best;
            prev_score = final_score;
        }
        if timer.timed_out(budget as u64) || depth_timer.elapsed_ms() > (budget as u64) / 2 { break; }
    }
    (best_move.1, best_move.2, best_move.3, best_move.4)
}

fn alpha_beta(
    board: &mut AgentBoard,
    state: &mut SearchState,
    depth: i16,
    mut alpha: i32,
    beta: i32,
    timer: &Timer,
    budget_ms: u64,
    best_move: &mut (i32, i8, i8, i8, i8),
    weights: &EvalWeights,
) -> i32 {
    if board.consecutive_passes >= 2 {
        let p = board.player;
        let opp = opponent(p);
        let margin = board.owned_cells(p) - board.owned_cells(opp);
        if margin > 0 { return 100000 + margin; }
        if margin < 0 { return -100000 + margin; }
        return 0;
    }

    if (unsafe { NODES_SEARCHED } & 4095) == 0 && timer.timed_out(budget_ms) {
        return evaluate(board, weights);
    }
    unsafe { NODES_SEARCHED += 1; }

    if depth == 0 {
        return evaluate(board, weights);
    }

    // TT probe
    let key = hash_board(&board.values, &board.owners, board.player);
    let (hit, tt_value, tt_move) = state.tt.probe(key, depth, alpha, beta);
    if hit {
        *best_move = (0, tt_move.0, tt_move.1, tt_move.2, tt_move.3);
        return tt_value;
    }

    let moves = legal_moves_prioritized(board);
    if moves.is_empty() {
        let record = board.make_move(-1, -1, -1, -1);
        let score = -alpha_beta(board, state, depth - 1, -beta, -alpha, timer, budget_ms, best_move, weights);
        board.unmake_move(&record);
        return score;
    }

    // Order: TT move first
    let tt_move_tuple = tt_move;
    let mut ordered = moves;
    ordered.sort_by_key(|m| if (m.1, m.2, m.3, m.4) == tt_move_tuple { i32::MAX } else { m.0 });

    let mut best_value = -999999;
    let mut local_best = ordered[0];
    let mut flag = TT_UPPER;
    let alpha_orig = alpha;

    for mv in &ordered {
        let record = board.make_move(mv.1, mv.2, mv.3, mv.4);
        let score = -alpha_beta(board, state, depth - 1, -beta, -alpha, timer, budget_ms, best_move, weights);
        board.unmake_move(&record);

        if score > best_value {
            best_value = score;
            local_best = *mv;
        }
        if score > alpha {
            alpha = score;
            flag = TT_EXACT;
        }
        if alpha >= beta {
            flag = TT_LOWER;
            break;
        }
    }

    // PASS consideration in low-mobility endgame
    if depth >= 3 && ordered.len() <= 5 {
        let static_eval = evaluate(board, weights);
        if static_eval > 0 {
            let record = board.make_move(-1, -1, -1, -1);
            let pass_score = -alpha_beta(board, state, depth - 1, -beta, -alpha, timer, budget_ms, best_move, weights);
            board.unmake_move(&record);
            if pass_score > best_value {
                best_value = pass_score;
                local_best = (0, -1, -1, -1, -1);
                flag = TT_EXACT;
            }
        }
    }

    *best_move = local_best;

    if flag == TT_UPPER && best_value > alpha_orig {
        flag = TT_EXACT;
    }

    state.tt.store(key, depth, best_value, flag, local_best.1, local_best.2, local_best.3, local_best.4);
    best_value
}
