mod board;
mod eval;
mod search;
mod protocol;
mod mcts;
mod timeman;
mod movegen;
mod data;

use crate::protocol::CordycepsProtocol;

fn main() {
    let mut proto = CordycepsProtocol::new();
    proto.run();
}
