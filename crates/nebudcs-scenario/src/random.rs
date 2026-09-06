//! Deterministic pseudo-random number generation for scenario disturbances and degradation.
//!
//! # Architectural Invariant
//!
//! In accordance with ADR-002 (Simulation Determinism), all pseudo-random processes
//! (e.g. sensor noise, signal degradation) MUST use an explicit, deterministic seed.
//! Uncontrolled system entropy (`rand::thread_rng()`, `/dev/urandom`, system clocks)
//! is strictly forbidden in the simulation and scenario pipeline.

/// Deterministic 64-bit XorShift pseudo-random number generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicPrng {
    state: u64,
}

impl DeterministicPrng {
    /// Creates a new PRNG with an explicit seed.
    /// Non-zero state is enforced (if seed is 0, defaults to 0x853c49e6748fea9b).
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x853c_49e6_748f_ea9b
        } else {
            seed
        };
        Self { state }
    }

    /// Generates the next pseudo-random 64-bit integer.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Generates the next pseudo-random float in the half-open interval `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        // Use upper 53 bits for standard double precision mantissa
        let bits = self.next_u64() >> 11;
        (bits as f64) / ((1u64 << 53) as f64)
    }

    /// Generates a bipolar float uniformly distributed in `[-1.0, 1.0)`.
    pub fn next_bipolar(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }

    /// Resets the generator state to a specific seed.
    pub fn reset(&mut self, seed: u64) {
        *self = Self::new(seed);
    }
}

impl Default for DeterministicPrng {
    fn default() -> Self {
        Self::new(0x42_1337_cafe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_repeatability() {
        let mut prng1 = DeterministicPrng::new(12345);
        let mut prng2 = DeterministicPrng::new(12345);

        for _ in 0..100 {
            assert_eq!(prng1.next_u64(), prng2.next_u64());
            assert_eq!(prng1.next_f64(), prng2.next_f64());
        }
    }

    #[test]
    fn test_range_bounds() {
        let mut prng = DeterministicPrng::new(999);
        for _ in 0..1000 {
            let val = prng.next_f64();
            assert!((0.0..1.0).contains(&val));
            let bip = prng.next_bipolar();
            assert!((-1.0..1.0).contains(&bip));
        }
    }
}
