use std::io::{self, BufRead, Write};

use crate::board::AgentBoard;
use crate::data::AgentData;
use crate::search::search_best_move;
use mushroom_bot::types::*;

pub struct Protocol {
    board: AgentBoard,
    i_am_first: bool,
    opp_consecutive_passes: usize,
    running: bool,
    data: AgentData,
}

impl Protocol {
    pub fn new() -> Self {
        Self {
            board: AgentBoard::from_rows(&vec!["00000000000000000".to_string(); ROWS]),
            i_am_first: false,
            opp_consecutive_passes: 0,
            running: true,
            data: AgentData::load(),
        }
    }

    pub fn handle(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() { return; }

        if line.starts_with("READY") {
            self.handle_ready(line);
        } else if line.starts_with("INIT") {
            self.handle_init(line);
        } else if line.starts_with("TIME") {
            self.handle_time(line);
        } else if line.starts_with("OPP") {
            self.handle_opp(line);
        } else if line.starts_with("FINISH") {
            self.running = false;
        }
    }

    fn write_line(msg: &str) {
        println!("{msg}");
        let _ = io::stdout().flush();
    }

    fn handle_ready(&mut self, line: &str) {
        self.i_am_first = line.contains("FIRST");
        self.board.set_player(if self.i_am_first { FIRST } else { SECOND });
        Self::write_line("OK");
    }

    fn handle_init(&mut self, line: &str) {
        // Format: "INIT row1 row2 ... row10"
        let board_str = &line[4..].trim();
        let parts: Vec<&str> = board_str.split_whitespace().collect();
        let rows: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
        if rows.len() >= ROWS {
            self.board = AgentBoard::from_rows(&rows[..ROWS]);
        }
        self.opp_consecutive_passes = 0;
    }

    fn handle_time(&mut self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 { return; }
        let _our_time: i64 = parts[1].parse().unwrap_or(0);
        let _opp_time: i64 = parts[2].parse().unwrap_or(0);

        // Count live mushrooms
        let mut live = 0i32;
        for v in &self.board.values { if *v > 0 { live += 1; } }
        let est_moves = (live / 4).max(4);
        let time_budget = (_our_time / est_moves as i64).clamp(20, 2500);

        // Double-pass lock-in
        if self.opp_consecutive_passes >= 2 {
            let margin = self.board.owned_cells(self.board.player)
                - self.board.owned_cells(opponent(self.board.player));
            if margin > 0 {
                Self::write_line("-1 -1 -1 -1");
                self.board.apply_move(-1, -1, -1, -1);
                return;
            }
        }

        let (r1, c1, r2, c2) = search_best_move(&self.board, time_budget, &self.data);

        if (r1, c1, r2, c2) == (-1, -1, -1, -1) {
            Self::write_line("-1 -1 -1 -1");
        } else {
            Self::write_line(&format!("{r1} {c1} {r2} {c2}"));
        }
        self.board.apply_move(r1, c1, r2, c2);
    }

    fn handle_opp(&mut self, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 { return; }
        let r1: i8 = parts[1].parse().unwrap_or(-1);
        let c1: i8 = parts[2].parse().unwrap_or(-1);
        let r2: i8 = parts[3].parse().unwrap_or(-1);
        let c2: i8 = parts[4].parse().unwrap_or(-1);

        if (r1, c1, r2, c2) == (-1, -1, -1, -1) {
            self.opp_consecutive_passes += 1;
        } else {
            self.opp_consecutive_passes = 0;
        }
        self.board.apply_move(r1, c1, r2, c2);
    }
}
