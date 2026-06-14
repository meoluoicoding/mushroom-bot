use std::io::{self, BufRead, Write};

mod protocol;
mod board;
mod eval;
mod search;
mod zobrist;
mod tt;
mod timer;
mod data;

use crate::protocol::Protocol;

fn main() {
    let mut proto = Protocol::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        proto.handle(&line);
    }
}
