//! Random number generation core implementations.
//!
//! Pure logic for rand package functions.

use core::sync::atomic::{AtomicU64, Ordering};

static SEED: AtomicU64 = AtomicU64::new(1);

/// Generate next random number using LCG algorithm.
fn next_random() -> u64 {
    let mut seed = SEED.load(Ordering::Relaxed);
    // LCG parameters from Numerical Recipes
    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    SEED.store(seed, Ordering::Relaxed);
    seed
}

/// Set random seed.
pub fn seed(s: i64) {
    SEED.store(s as u64, Ordering::Relaxed);
}

/// Generate random non-negative int64.
pub fn int() -> i64 {
    (next_random() >> 1) as i64
}

/// Generate random non-negative int64.
pub fn int64() -> i64 {
    (next_random() >> 1) as i64
}

/// Generate random int in [0, n).
pub fn intn(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    ((next_random() >> 1) as i64) % n
}

/// Generate random int63 in [0, n).
pub fn int63n(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    ((next_random() >> 1) as i64) % n
}

/// Generate random float64 in [0.0, 1.0).
pub fn float64() -> f64 {
    (next_random() >> 11) as f64 / (1u64 << 53) as f64
}

/// Generate random float32 in [0.0, 1.0).
pub fn float32() -> f32 {
    (next_random() >> 40) as f32 / (1u64 << 24) as f32
}

/// Generate permutation of [0, n) using Fisher-Yates shuffle.
/// Returns the permuted indices.
pub fn perm(n: usize) -> alloc::vec::Vec<usize> {
    let mut result: alloc::vec::Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = ((next_random() >> 1) as usize) % (i + 1);
        result.swap(i, j);
    }
    result
}
