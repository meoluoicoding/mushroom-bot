use std::io::{self, BufRead, Write};

use mushroom_bot::board::Board;
use mushroom_bot::dataloader::{load_data_bin, read_weights_from_txt, EvalWeights, GameData};
use mushroom_bot::search::{Search, SearchConfig};
use mushroom_bot::side::{GameSide, SideTuning};
use mushroom_bot::timeman::{SearchPhase, TimeManager};
use mushroom_bot::types::*;

struct Agent {
    board: Option<Board>,
    my_player: i8,
    side: GameSide,
    side_tuning: SideTuning,
    time_mgr: TimeManager,
    search: Search,
    trace_search: bool,
    opp_consecutive_passes: u32,
    opp_passes_since_our_move: u32,
}

impl Agent {
    fn new() -> Self {
        // Try to load game data from data.bin
        let (weights, game_data) = Self::load_weights_and_data();
        eprintln!("LOADED WEIGHTS: {:?}", weights);
        eprintln!("LOADED GameData: {}; geometry rects: {}", game_data.is_some(), game_data.as_ref().map(|gd| gd.geometry.len()).unwrap_or(0));
        let base_config = Self::search_config_from_env();

        let search = if let Some(gd) = game_data {
            Search::with_game_data(base_config, gd)
        } else {
            Search::with_weights(base_config, weights)
        };

        Self {
            board: None,
            my_player: FIRST,
            side: GameSide::First,
            side_tuning: GameSide::First.tuning(),
            time_mgr: TimeManager::new(),
            search,
            trace_search: std::env::var("MUSHROOM_TRACE_SEARCH").is_ok(),
            opp_consecutive_passes: 0,
            opp_passes_since_our_move: 0,
        }
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

        // Try text files
        for path in &["balanced.txt", "weights.txt", "attacker.txt", "defender.txt"] {
            if let Some(w) = read_weights_from_txt(path) {
                return (w, None);
            }
        }

        // Use defaults without external training artifacts.
        (EvalWeights::default(), Some(GameData::default()))
    }

    fn search_config_from_env() -> SearchConfig {
        fn env_bool(name: &str, default: bool) -> bool {
            match std::env::var(name) {
                Ok(value) => match value.to_ascii_lowercase().as_str() {
                    "1" | "true" | "yes" | "on" => true,
                    "0" | "false" | "no" | "off" => false,
                    _ => default,
                },
                Err(_) => default,
            }
        }

        SearchConfig {
            time_budget_ms: 100,
            use_tt: env_bool("MUSHROOM_USE_TT", true),
            use_ordering: env_bool("MUSHROOM_USE_ORDERING", true),
            use_second_bonus: env_bool("MUSHROOM_USE_SECOND_BONUS", true),
            use_aspiration: true,
            use_mcts: false,
            use_qsearch: env_bool("MUSHROOM_USE_QSEARCH", true),
            use_lmr: env_bool("MUSHROOM_USE_LMR", true),
            use_futility: env_bool("MUSHROOM_USE_FUTILITY", true),
            use_mquality: env_bool("MUSHROOM_USE_MQUALITY", true),
            use_exact_endgame: env_bool("MUSHROOM_USE_EXACT_ENDGAME", true),
            // Keep these off by default; they have not been validated well enough yet.
            use_nmp: false,
            use_singular_extension: false,
            use_mtd: env_bool("MUSHROOM_USE_MTD", false),
        }
    }

    fn handle(&mut self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "READY" => {
                if parts.len() >= 2 && parts[1] != "FIRST" && parts[1] != "SECOND" {
                    eprintln!("WARNING: unrecognized READY side: {}", parts[1]);
                }
                self.my_player = if parts.len() < 2 || parts[1] == "FIRST" {
                    FIRST
                } else {
                    SECOND
                };
                self.side = GameSide::from_player(self.my_player);
                self.side_tuning = self.side.tuning();
                Some("OK".to_string())
            }
            "INIT" => {
                let mut rows: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                while rows.len() < ROWS {
                    let mut buf = String::new();
                    if io::stdin()
                        .read_line(&mut buf)
                        .ok()
                        .filter(|n| *n > 0)
                        .is_none()
                    {
                        break;
                    }
                    let trimmed = buf.trim().to_string();
                    if !trimmed.is_empty() {
                        rows.extend(trimmed.split_whitespace().map(|s| s.to_string()));
                    }
                }
                rows.truncate(ROWS);
                if rows.len() != ROWS || rows.iter().any(|r| r.len() != COLS) {
                    eprintln!("WARNING: invalid INIT dimensions: {} rows (expected {}x{})", rows.len(), ROWS, COLS);
                }
                self.board = Some(Board::from_rows(&rows));
                self.time_mgr = TimeManager::new();
                self.opp_consecutive_passes = 0;
                self.opp_passes_since_our_move = 0;
                if self.side_tuning.clear_tt_on_init() {
                    self.search.clear_tt();
                }
                self.search.rect_cache.clear();
                None
            }
            "TIME" => {
                let state = match &self.board {
                    Some(b) if !b.is_terminal() => b.clone(),
                    _ => {
                        eprintln!("WARNING: TIME called on terminal/empty board");
                        return Some(format_move(PASS));
                    }
                };
                self.opp_passes_since_our_move = 0;
                if self.opp_consecutive_passes >= 2 {
                    let my_score = state.my_mask.popcount() as i32;
                    let opp_score = state.opp_mask.popcount() as i32;
                    if my_score > opp_score {
                        self.board = Some(state.apply_action(PASS));
                        return Some(format_move(PASS));
                    }
                }
                let my = parse_int(parts.get(1)).unwrap_or(0);
                let opp = parse_int(parts.get(2)).unwrap_or(0);
                self.time_mgr.update(my, opp);

                let live = state.live_mask.popcount();
                let root_key = Search::rect_cache_key(&state.values);
                let rects = self.search.cached_rectangles(root_key, &state.values);
                let budget = self.time_mgr.search_budget_ms(live, rects.len());
                let phase = self.time_mgr.phase(live, rects.len());

                self.search.config = SearchConfig {
                    time_budget_ms: budget.max(1),
                    use_aspiration: matches!(phase, SearchPhase::MidgameFull | SearchPhase::Endgame),
                    ..self.search.config
                };
                self.search.config = self.side_tuning.search_config(self.search.config, phase);
                self.search.set_side(self.side);

                let result = self.search.think(&state);
                if self.trace_search {
                    eprintln!(
                        "TRACE side={:?} depth={} max_ply={} value={:.2} nodes={} elapsed_ms={:.1}",
                        self.side,
                        result.depth,
                        result.max_ply_reached,
                        result.value,
                        result.nodes,
                        result.elapsed_ms
                    );
                }
                let mv = if state.is_legal_action(result.action) {
                    result.action
                } else {
                    eprintln!("WARNING: search returned illegal action: {:?}, falling back to PASS", result.action);
                    PASS
                };
                self.board = Some(state.apply_action(mv));
                self.opp_passes_since_our_move = 0;
                Some(format_move(mv))
            }
            "OPP" => {
                if let Some(ref bd) = self.board {
                    if !bd.is_terminal() && parts.len() >= 5 {
                        if let Ok(mv) = parse_move(&parts[1..5]) {
                            if bd.is_legal_action(mv) {
                                if mv == PASS {
                                    self.opp_consecutive_passes += 1;
                                    self.opp_passes_since_our_move += 1;
                                    self.board = Some(bd.apply_action(mv));
                                } else {
                                    self.opp_consecutive_passes = 0;
                                    self.opp_passes_since_our_move = 0;
                                    self.board = Some(bd.apply_action(mv));
                                }
                            } else {
                                eprintln!("WARNING: opponent chose illegal action: {:?}", mv);
                            }
                        } else {
                            eprintln!("WARNING: failed to parse opponent move: {:?}", &parts[1..5]);
                        }
                    }
                }
                None
            }
            "FINISH" => std::process::exit(0),
            _ => None,
        }
    }
}

fn format_move(mv: Move) -> String {
    format!("{} {} {} {}", mv.0, mv.1, mv.2, mv.3)
}

fn parse_move(parts: &[&str]) -> Result<Move, ()> {
    if parts.len() < 4 {
        return Err(());
    }
    Ok((
        parts[0].parse::<i8>().map_err(|_| ())?,
        parts[1].parse::<i8>().map_err(|_| ())?,
        parts[2].parse::<i8>().map_err(|_| ())?,
        parts[3].parse::<i8>().map_err(|_| ())?,
    ))
}

fn parse_int(v: Option<&&str>) -> Option<u64> {
    v.and_then(|s| s.parse::<u64>().ok())
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut agent = Agent::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(resp) = agent.handle(&line) {
            let mut out = stdout.lock();
            let _ = writeln!(out, "{}", resp);
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_board() -> Board {
        let rows: Vec<String> = vec![
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
            "12345678912345678".to_string(),
        ];
        Board::from_rows(&rows)
    }

    #[test]
    fn test_opp_consecutive_passes_update_board() {
        let mut agent = Agent::new();
        agent.board = Some(make_test_board());
        agent.opp_consecutive_passes = 0;
        agent.opp_passes_since_our_move = 0;

        let _ = agent.handle("OPP -1 -1 -1 -1 0");
        let after_first = agent.board.as_ref().unwrap().clone();
        assert_eq!(after_first.passes, 1);
        assert_eq!(agent.opp_consecutive_passes, 1);
        assert_eq!(agent.opp_passes_since_our_move, 1);

        let _ = agent.handle("OPP -1 -1 -1 -1 0");
        let after_second = agent.board.as_ref().unwrap().clone();
        assert_eq!(after_second.passes, 2);
        assert!(after_second.is_terminal());
        assert_eq!(agent.opp_consecutive_passes, 2);
        assert_eq!(agent.opp_passes_since_our_move, 2);
    }
}
