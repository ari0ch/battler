/// A small seeded pseudo-random number generator (SplitMix64).
///
/// The sim has no dependencies, so we bring our own RNG. SplitMix64 is tiny,
/// fast and statistically decent; it is not cryptographically secure, which
/// doesn't matter for a game. The same seed always yields the same sequence,
/// which is what makes battles reproducible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        // `wrapping_*` makes overflow explicit: in debug builds a plain `+` or `*`
        // that overflows would panic, but here wrap-around is exactly what we want.
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (`n` must be non-zero).
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "Rng::below(0)");
        (self.next_u64() % u64::from(n)) as u32
    }

    /// Shuffles a slice in place (Fisher-Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }

    /// Returns true with roughly `percent`% probability.
    ///
    /// (`% 100` has a negligible bias for a 64-bit source; fine for gameplay.)
    pub fn chance(&mut self, percent: u32) -> bool {
        self.next_u64() % 100 < u64::from(percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn chance_is_roughly_calibrated() {
        let mut rng = Rng::new(7);
        let hits = (0..10_000).filter(|_| rng.chance(15)).count();
        assert!((1_300..1_700).contains(&hits), "got {hits} hits");
    }
}
