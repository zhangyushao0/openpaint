//! The seeds strokes' stamps jitter by (S45, DECISIONS D72): every stroke drawn on the device
//! takes its own from a small generator started from the wall clock; the harness fixes one seed
//! for every stroke, so a scenario's hashes hold and a stroke drawn twice lands twice alike.

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct Seeds {
    state: u64,
    fixed: Option<u32>,
}

impl Seeds {
    /// Started from the wall clock's nanoseconds.
    pub(crate) fn from_clock() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        Self {
            state: nanos as u64,
            fixed: None,
        }
    }

    /// Every stroke from now on takes `seed`.
    pub(crate) const fn fix(&mut self, seed: u32) {
        self.fixed = Some(seed);
    }

    /// The next stroke's seed (SplitMix64's step, its high half).
    pub(crate) fn next(&mut self) -> u32 {
        if let Some(seed) = self.fixed {
            return seed;
        }
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strokes_take_seeds_of_their_own_unless_fixed() {
        let mut seeds = Seeds {
            state: 7,
            fixed: None,
        };
        let drawn: Vec<u32> = (0..100).map(|_| seeds.next()).collect();
        let mut distinct = drawn.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), drawn.len(), "every stroke its own");
        seeds.fix(0x5EED);
        assert_eq!([seeds.next(), seeds.next()], [0x5EED; 2]);
    }
}
