mod context;
mod keys;
mod protocol;
mod rng;
mod search;
mod styles;
mod utils;

use std::io::{self, BufRead, Write};

use crate::protocol::ZooProtocolBot;

fn main() {
    let mut mode = "greedy_area".to_string();
    let mut seed = 42u64;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                mode = args.get(i).cloned().unwrap_or_else(|| "greedy_area".to_string());
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(42);
            }
            _ => {}
        }
        i += 1;
    }

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut bot = ZooProtocolBot::new(mode, seed);

    let mut line = String::new();
    loop {
        line.clear();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if let Some(response) = bot.handle_command(&line, &mut input) {
            let _ = writeln!(output, "{response}");
            let _ = output.flush();
        }
    }
}
