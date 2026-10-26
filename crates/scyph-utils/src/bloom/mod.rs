//! Probabilistic set membership filters (Bloom filters).
//!
//! Two providers share the same hashing and sizing:
//!
//! - [`BloomFilter`] / [`SharedBloomFilter`]: in-process, no I/O.
//! - [`ValkeyBloomFilter`]: bits stored in Valkey/Redis (bitmap + Lua, no module required).
//!
//! # Examples
//!
//! ```rust
//! use scyph_utils::bloom::BloomFilter;
//!
//! let mut filter = BloomFilter::new(1000, 0.01);
//! filter.insert(&42);
//! assert!(filter.contains(&42));
//! ```
//!
//! ```rust,no_run
//! use scyph_utils::bloom::ValkeyBloomFilter;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let filter = ValkeyBloomFilter::from_env("crm:emails", 1_000_000, 0.01)?;
//! filter.add(&"user@example.com").await?;
//! assert!(filter.contains(&"user@example.com").await?);
//! # Ok(())
//! # }
//! ```

mod memory;
mod valkey;

pub use memory::{BloomFilter, SharedBloomFilter};
pub use valkey::{ValkeyBloomError, ValkeyBloomFilter};

use std::hash::{Hash, Hasher};

/// Upper bound on the number of bits a filter may allocate.
pub(crate) const MAX_BITS: usize = 1 << 28;

pub(crate) fn bloom_params(expected_items: usize, false_positive_rate: f64) -> (usize, u32) {
    let expected_items = expected_items.max(1);
    let fpr = if false_positive_rate.is_finite() {
        false_positive_rate.clamp(0.0001, 0.5)
    } else {
        0.01
    };
    let ln2 = std::f64::consts::LN_2;
    let m = (-(expected_items as f64 * fpr.ln()) / (ln2 * ln2)).ceil() as usize;
    let m = m.max(64);
    let k = ((m as f64 / expected_items as f64) * ln2).round() as u32;
    let k = k.clamp(1, 32);
    (m, k)
}

pub(crate) fn hash_pair<T: Hash>(item: &T) -> (u64, u64) {
    (fnv(item, 0xcbf29ce484222325), fnv(item, 0x84222325cbf29ce4))
}

pub(crate) fn hash_indices(
    h1: u64,
    h2: u64,
    bit_len: usize,
    hash_count: u32,
) -> impl Iterator<Item = usize> {
    let bit_len = bit_len as u64;
    // Make h2 coprime to bit_len so the k positions do not cycle early.
    let mut h2 = h2 % bit_len;
    if h2 == 0 {
        h2 = 1;
    }
    while gcd(h2, bit_len) != 1 {
        h2 += 1;
        if h2 >= bit_len {
            h2 = 1;
        }
    }
    (0..hash_count)
        .map(move |i| h1.wrapping_add((i as u64).wrapping_mul(h2)) as usize % (bit_len as usize))
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn fnv<T: Hash>(item: &T, seed: u64) -> u64 {
    let mut hasher = FnvHasher { hash: seed };
    item.hash(&mut hasher);
    if hasher.hash == 0 { 1 } else { hasher.hash }
}

struct FnvHasher {
    hash: u64,
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= *byte as u64;
            self.hash = self.hash.wrapping_mul(0x100000001b3);
        }
    }
}
