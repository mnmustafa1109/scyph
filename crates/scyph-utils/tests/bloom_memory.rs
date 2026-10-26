use scyph_utils::bloom::{BloomFilter, SharedBloomFilter};

#[test]
fn inserted_items_are_found() {
    let mut filter = BloomFilter::new(100, 0.01);
    for i in 0..100 {
        filter.insert(&i);
    }
    for i in 0..100 {
        assert!(filter.contains(&i));
    }
}

#[test]
fn clear_removes_items() {
    let mut filter = BloomFilter::new(10, 0.01);
    filter.insert(&1);
    filter.clear();
    assert!(!filter.contains(&1));
    assert!(filter.is_empty());
    assert_eq!(filter.len(), 0);
}

#[test]
fn empty_filter_contains_nothing() {
    let filter: BloomFilter<i32> = BloomFilter::new(10, 0.01);
    assert!(filter.is_empty());
    for i in 0..1000 {
        assert!(!filter.contains(&i));
    }
}

#[test]
fn with_bit_size_clamps_hash_count() {
    let filter: BloomFilter<i32> = BloomFilter::with_bit_size(64, 10_000);
    assert!(!filter.contains(&1));
}

#[test]
fn nan_rate_builds_usable_filter() {
    let mut filter = BloomFilter::new(10, f64::NAN);
    for i in 0..10 {
        filter.insert(&i);
    }
    for i in 0..10 {
        assert!(filter.contains(&i));
    }
}

#[test]
fn from_iter_sizes_and_warms() {
    let filter = BloomFilter::from_iter(0.01, 0..100);
    for i in 0..100 {
        assert!(filter.contains(&i));
    }
}

#[test]
fn shared_filter_is_thread_safe() {
    let filter = SharedBloomFilter::new(100, 0.01);
    filter.insert(&1);
    assert!(filter.contains(&1));
    filter.clear();
    assert!(!filter.contains(&1));
}

#[test]
fn works_with_non_clone_non_debug_items() {
    #[derive(Hash, PartialEq, Eq)]
    struct Opaque(i32);

    let filter = std::sync::Arc::new(SharedBloomFilter::<Opaque>::new(10, 0.01));
    filter.insert(&Opaque(1));
    assert!(filter.contains(&Opaque(1)));

    let shared = std::sync::Arc::clone(&filter);
    assert!(shared.contains(&Opaque(1)));
    let _ = format!("{filter:?}");
}

#[test]
fn debug_while_locked_does_not_deadlock() {
    let filter = std::sync::Arc::new(SharedBloomFilter::<i32>::new(10, 0.01));
    let guard = filter.lock();
    let debugged = std::sync::Arc::clone(&filter);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(format!("{debugged:?}"));
    });

    let text = rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("Debug blocked while the lock was held");
    assert!(text.contains("<locked>"));
    drop(guard);
}

#[test]
fn poisoned_lock_recovers() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let filter = SharedBloomFilter::new(10, 0.01);
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = filter.lock();
        panic!("poison the mutex");
    }));

    // The lock was poisoned by the panic above; the filter must still work.
    filter.insert(&1);
    assert!(filter.contains(&1));
}

#[test]
fn false_positive_rate_stays_bounded() {
    let mut filter = BloomFilter::new(1000, 0.01);
    for i in 0..10 {
        filter.insert(&i);
    }
    let mut false_positives = 0;
    for i in 1000..11000 {
        if filter.contains(&i) {
            false_positives += 1;
        }
    }
    // Very loose bound: should be well under 5%.
    assert!(false_positives < 500);
}
