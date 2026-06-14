use std::io::{self, BufRead, Write};
use mushroom_bot::board::Board;
use mushroom_bot::types::*;
use crate::search::find_best_move;

pub struct FinderProtocol {
    board: Option<Board>,
    my_player: i8,
    opp_consecutive_passes: u32,
    opp_passes_since_our_move: u32,
}

impl FinderProtocol {
    pub fn new() -> Self {
        Self { board: None, my_player: FIRST, opp_consecutive_passes: 0, opp_passes_since_our_move: 0 }
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }
            match parts[0] {
                "READY" => {
                    self.my_player = if parts.get(1).copied() == Some("SECOND") { SECOND } else { FIRST };
                    Self::write("OK");
                }
                "INIT" => {
                    let mut rows: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                    let mut input = stdin.lock();
                    while rows.len() < ROWS {
                        let mut buf = String::new();
                        if input.read_line(&mut buf).ok().filter(|n| *n > 0).is_none() { break; }
                        let t = buf.trim().to_string();
                        if !t.is_empty() { rows.extend(t.split_whitespace().map(|s| s.to_string())); }
                    }
                    rows.truncate(ROWS);
                    self.board = Some(Board::from_rows(&rows));
                    self.opp_consecutive_passes = 0;
                    self.opp_passes_since_our_move = 0;
                }
                "TIME" => {
                    let state = match &self.board {
                        Some(b) if !b.is_terminal() => b.clone(),
                        _ => { Self::write("-1 -1 -1 -1"); continue; }
                    };
                    self.opp_passes_since_our_move = 0;

                    let my_ms: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let live = state.live_mask.popcount() as u64;
                    let est = (live / 4).max(4);
                    let budget = (my_ms.saturating_sub(500) / est).clamp(20, 2500);

                    // Double-pass win lock
                    if self.opp_consecutive_passes >= 2 {
                        let my_score = state.my_mask.popcount() as i32;
                        let opp_score = state.opp_mask.popcount() as i32;
                        if my_score > opp_score {
                            self.board = Some(state.apply_action(PASS));
                            Self::write("-1 -1 -1 -1");
                            continue;
                        }
                    }

                    let (r1, c1, r2, c2) = find_best_move(&state, budget);
                    let mv = if state.is_legal_action((r1, c1, r2, c2)) { (r1, c1, r2, c2) } else { PASS };
                    self.board = Some(state.apply_action(mv));
                    Self::write(&format!("{} {} {} {}", mv.0, mv.1, mv.2, mv.3));
                }
                "OPP" => {
                    if let Some(bd) = &self.board {
                        if !bd.is_terminal() && parts.len() >= 5 {
                            let r1: i8 = parts[1].parse().unwrap_or(-1);
                            let c1: i8 = parts[2].parse().unwrap_or(-1);
                            let r2: i8 = parts[3].parse().unwrap_or(-1);
                            let c2: i8 = parts[4].parse().unwrap_or(-1);
                            let mv = (r1, c1, r2, c2);
                            if !bd.is_legal_action(mv) { continue; }
                            if mv == PASS {
                                self.opp_consecutive_passes += 1;
                                self.opp_passes_since_our_move += 1;
                                // Artifact detection: skip 2nd+ consecutive opp pass
                                if self.opp_passes_since_our_move <= 1 {
                                    self.board = Some(bd.apply_action(mv));
                                }
                            } else {
                                self.opp_consecutive_passes = 0;
                                self.opp_passes_since_our_move = 0;
                                self.board = Some(bd.apply_action(mv));
                            }
                        }
                    }
                }
                "FINISH" => std::process::exit(0),
                _ => {}
            }
        }
    }

    fn write(msg: &str) {
        let _ = writeln!(io::stdout(), "{}", msg);
        let _ = io::stdout().flush();
    }
}
