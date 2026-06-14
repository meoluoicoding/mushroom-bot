use std::io::{self, BufRead, Write};
use mushroom_bot::board::Board;
use mushroom_bot::types::*;
use crate::data::PolicyData;
use crate::search::choose_action;

pub struct PolicyProtocol {
    board: Option<Board>,
    my_player: i8,
    my_time_left_ms: u64,
    data: PolicyData,
}

impl PolicyProtocol {
    pub fn new() -> Self {
        Self { board: None, my_player: FIRST, my_time_left_ms: 10000, data: PolicyData::load() }
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
                }
                "TIME" => {
                    let state = match &self.board {
                        Some(b) if !b.is_terminal() => b.clone(),
                        _ => { Self::write("-1 -1 -1 -1"); continue; }
                    };
                    self.my_time_left_ms = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10000);
                    let (r1, c1, r2, c2) = choose_action(&state, self.my_time_left_ms, &self.data);
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
                            if bd.is_legal_action((r1, c1, r2, c2)) {
                                self.board = Some(bd.apply_action((r1, c1, r2, c2)));
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
