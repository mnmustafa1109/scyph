//! In-process Bloom filter.

use super::{MAX_BITS, bloom_params, hash_indices, hash_pair};
use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Mutex, MutexGuard};

/// A Bloom filter for approximate set membership.
///
/// A Bloom filter can tell you that an item is *definitely not* in a set, or
/// that it is *possibly* in the set. False positives are possible; false
/// negatives are not. Individual items cannot be removed; call [`BloomFilter::clear`]
/// to reset the whole filter.
///
/// This type is not internally synchronized. Use [`SharedBloomFilter`] when
/// sharing one instance across tasks.
///
/// # Example
///
/// ```rust
/// use scyph_utils::bloom::BloomFilter;
///
/// let mut filter = BloomFilter::new(1000, 0.01);
/// filter.insert(&42);
/// assert!(filter.contains(&42));
/// ```
pub struct BloomFilter<T> {
    bits: Vec<u64>,
    hash_count: u32,
    item_count: usize,
    _marker: PhantomData<T>,
}

impl<T> Clone for BloomFilter<T> {
    fn clone(&self) -> Self {
        Self {
            bits: self.bits.clone(),
            hash_count: self.hash_count,
            item_count: self.item_count,
            _marker: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for BloomFilter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloomFilter")
            .field("bits", &(self.bits.len() * 64))
            .field("hash_count", &self.hash_count)
            .field("item_count", &self.item_count)
            .finish()
    }
}

impl<T: Hash> BloomFilter<T> {
    /// Creates a filter tuned for `expected_items` with the given false-positive rate.
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let (m, k) = bloom_params(expected_items, false_positive_rate);
        Self::with_bit_size(m, k)
    }

    /// Creates a filter with an explicit bit count and hash count.
    ///
    /// The bit count is rounded up to a multiple of 64. The hash count is
    /// clamped to the range `1..=32`. The bit count is capped at
    /// `MAX_BITS` to avoid multi-gigabyte allocations from a bad value.
    pub fn with_bit_size(bits: usize, hash_count: u32) -> Self {
        let bits = bits.clamp(64, MAX_BITS);
        let words = bits.div_ceil(64);
        Self {
            bits: vec![0; words],
            hash_count: hash_count.clamp(1, 32),
            item_count: 0,
            _marker: PhantomData,
        }
    }

    /// Inserts many items into the filter (useful for warming from a DB result set).
    pub fn insert_many<U>(&mut self, items: impl IntoIterator<Item = U>)
    where
        U: Borrow<T>,
    {
        for item in items {
            self.insert(item.borrow());
        }
    }

    /// Creates a filter sized to the iterator and warms it.
    ///
    /// Collects the iterator once to learn its length, so peak memory is the
    /// full item set. For very large sets, prefer [`BloomFilter::new`] plus
    /// [`BloomFilter::insert_many`].
    pub fn from_iter<U, I>(false_positive_rate: f64, items: I) -> Self
    where
        I: IntoIterator<Item = U>,
        U: Borrow<T>,
    {
        let items: Vec<U> = items.into_iter().collect();
        let mut filter = Self::new(items.len().max(1), false_positive_rate);
        filter.insert_many(items);
        filter
    }

    /// Inserts an item into the filter.
    pub fn insert(&mut self, item: &T) {
        let (h1, h2) = hash_pair(item);
        let bit_len = self.bits.len() * 64;
        for index in hash_indices(h1, h2, bit_len, self.hash_count) {
            self.bits[index / 64] |= 1u64 << (index % 64);
        }
        self.item_count = self.item_count.saturating_add(1);
    }

    /// Returns `true` if the item might be in the set.
    pub fn contains(&self, item: &T) -> bool {
        let (h1, h2) = hash_pair(item);
        let bit_len = self.bits.len() * 64;
        hash_indices(h1, h2, bit_len, self.hash_count)
            .all(|index| self.bits[index / 64] & (1u64 << (index % 64)) != 0)
    }

    /// Removes all items from the filter.
    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.item_count = 0;
    }

    /// Number of inserted items (approximate; duplicates are counted).
    pub fn len(&self) -> usize {
        self.item_count
    }

    /// Returns `true` if no items have been inserted.
    pub fn is_empty(&self) -> bool {
        self.item_count == 0
    }
}

/// A thread-safe wrapper around [`BloomFilter`] for sharing across tasks.
///
/// Share it by wrapping it in an [`std::sync::Arc`]. This wrapper is not
/// `Clone` (a clone would duplicate the filter, not share it).
///
/// ```rust
/// use scyph_utils::bloom::SharedBloomFilter;
///
/// let filter = SharedBloomFilter::new(1000, 0.01);
/// filter.insert(&42);
/// assert!(filter.contains(&42));
/// ```
pub struct SharedBloomFilter<T> {
    inner: Mutex<BloomFilter<T>>,
}

impl<T> std::fmt::Debug for SharedBloomFilter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not block on the lock here: logging while holding a guard must not deadlock.
        let guard = match self.inner.try_lock() {
            Ok(g) => Some(g),
            Err(std::sync::TryLockError::Poisoned(e)) => Some(e.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => None,
        };
        match guard {
            Some(inner) => f
                .debug_struct("SharedBloomFilter")
                .field("inner", &*inner)
                .finish(),
            None => f
                .debug_struct("SharedBloomFilter")
                .field("inner", &"<locked>")
                .finish(),
        }
    }
}

impl<T: Hash> SharedBloomFilter<T> {
    /// Creates a new shared filter.
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        Self {
            inner: Mutex::new(BloomFilter::new(expected_items, false_positive_rate)),
        }
    }

    /// Inserts an item.
    pub fn insert(&self, item: &T) {
        self.lock().insert(item);
    }

    /// Returns `true` if the item might be in the set.
    pub fn contains(&self, item: &T) -> bool {
        self.lock().contains(item)
    }

    /// Removes all items.
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Access the inner filter under a lock (for bulk operations).
    ///
    /// Recovers the guard if another thread panicked while holding it, so one
    /// failed request cannot poison the filter for the whole process.
    pub fn lock(&self) -> MutexGuard<'_, BloomFilter<T>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}
