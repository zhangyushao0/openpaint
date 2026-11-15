//! The generators' randomness: SplitMix64, the same numbers for the same seed on every machine
//! and in every version, so an exercise is its kind and its seed.

/// A SplitMix64 sequence.
#[derive(Debug, Clone)]
pub struct Random(u64);

impl Random {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next 64 bits.
    pub const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Evenly in `0..1`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Evenly in `low..high`.
    pub fn range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }

    /// 1 or -1, as likely.
    pub const fn sign(&mut self) -> f64 {
        if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 }
    }
}

/// The seed of item `index` of something seeded `seed` (an exercise of a lesson).
#[must_use]
pub const fn derived(seed: u64, index: u64) -> u64 {
    Random::new(seed ^ index.wrapping_mul(0xA24B_AED4_963E_E407)).next_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_gives_its_known_numbers_and_stays_in_range() {
        // SplitMix64 from 0, as its reference implementation gives it.
        let mut random = Random::new(0);
        assert_eq!(random.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(random.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        let mut random = Random::new(7);
        for _ in 0..1000 {
            let value = random.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&value));
        }
        assert_ne!(derived(1, 0), derived(1, 1));
        assert_eq!(derived(9, 4), derived(9, 4));
    }
}
