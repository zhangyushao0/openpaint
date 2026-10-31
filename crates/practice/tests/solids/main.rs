//! Solids in boxes (S93): the exact answer on generated boxes (V大's construction on the picture
//! lands on it, the sides touch the ends, a sphere's outline is its silhouette and his circle lies
//! within the pen's width of it) and the copy read back from simulated hands (S84's).

#[expect(
    dead_code,
    reason = "S84's hand: the solids are drawn with its lines alone"
)]
#[path = "../boxes/hand.rs"]
mod hand;

mod exact;
mod hands;
mod off;
