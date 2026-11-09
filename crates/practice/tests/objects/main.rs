//! Objects simplified into boxes (S94): the exact answer and V大's construction on generated
//! objects, the picture against the answer, and the copy read back from simulated hands (S84's).

#[expect(
    dead_code,
    reason = "S84's hand: the objects are drawn with its lines alone"
)]
#[path = "../boxes/hand.rs"]
mod hand;

mod exact;
mod hands;
mod off;
mod picture;
mod verdict;
