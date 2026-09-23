//! 让大肥鱼写了一个从系统熵生成随机数的模块
//! （虽然好像直接用现成的crate也可以）
//! （算了代码能跑起来就不要考虑这么多了owo）
//!
//! A tiny deterministic random number generator.
//!
//! 2048 needs exactly one random decision per move — which empty cell a new tile
//! appears in, and whether it is a `2` or a `4` — so a hand-rolled generator is
//! cheaper than a dependency and, more usefully, makes games reproducible from a
//! seed in tests.

use std::time::{SystemTime, UNIX_EPOCH};

/// SplitMix64: a small, fast, well-distributed generator.
///
/// Statistical quality far beyond what picking a cell out of 16 needs, but it is
/// short enough to audit in one screen and has no state to corrupt.
#[derive(Debug, Clone, Copy)]
pub struct Rng {
    state: u64,
}

/// The golden-ratio odd constant SplitMix64 increments its state by.
const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

impl Rng {
    /// Builds a generator from an explicit seed. The same seed always produces
    /// the same sequence, which is what makes tests deterministic.
    pub const fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Builds a generator seeded from the clock.
    pub fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos() as u64);
        // Mixing with GAMMA keeps a low-entropy clock reading from producing a
        // weakly seeded first output.
        Self::from_seed(nanos ^ GAMMA)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`. Returns `0` when `bound` is `0`.
    ///
    /// The modulo bias is irrelevant for a 16-cell board and keeps this readable.
    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as usize
    }

    /// `true` with probability `numerator / denominator`.
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        debug_assert!(numerator <= denominator, "probability cannot exceed 1");
        debug_assert!(denominator > 0, "probability needs a denominator");
        (self.next_u64() % u64::from(denominator)) < u64::from(numerator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_the_same_sequence() {
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);
        for _ in 0..32 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::from_seed(1);
        let mut b = Rng::from_seed(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn below_stays_in_range_and_reaches_both_ends() {
        let mut rng = Rng::from_seed(7);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1_000 {
            let value = rng.below(16);
            assert!(value < 16, "{value} out of range");
            seen.insert(value);
        }
        assert_eq!(seen.len(), 16, "every cell should be reachable");
    }

    #[test]
    fn below_zero_is_zero() {
        let mut rng = Rng::from_seed(7);
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn chance_is_roughly_the_requested_probability() {
        let mut rng = Rng::from_seed(2024);
        let hits = (0..10_000).filter(|_| rng.chance(9, 10)).count();
        // 1% tolerance is far wider than the noise at this sample size.
        assert!((8_900..=9_100).contains(&hits), "got {hits} hits in 10000");
    }

    #[test]
    fn chance_one_is_always_true_and_zero_never_true() {
        let mut rng = Rng::from_seed(11);
        for _ in 0..100 {
            assert!(rng.chance(1, 1));
            assert!(!rng.chance(0, 1));
        }
    }

    #[test]
    fn entropy_seeded_generators_differ() {
        // Two calls in quick succession must still diverge; a plain nanosecond
        // seed could collide on a coarse clock.
        let mut a = Rng::from_entropy();
        let mut b = Rng::from_entropy();
        assert_ne!(a.next_u64(), b.next_u64());
    }
}
