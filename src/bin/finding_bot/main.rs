mod search;
mod protocol;
use crate::protocol::FinderProtocol;

fn main() {
    let mut proto = FinderProtocol::new();
    proto.run();
}
