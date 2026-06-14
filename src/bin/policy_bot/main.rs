mod search;
mod protocol;
mod data;
use crate::protocol::PolicyProtocol;

fn main() {
    let mut proto = PolicyProtocol::new();
    proto.run();
}
