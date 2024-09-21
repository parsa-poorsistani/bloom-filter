use bloomf::{AtomicBloomFilter, BloomFilter, ThreadSafeBF};
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use std::thread;

fn bench_bloom_filter(c: &mut Criterion) {
    // Bit array sizes for benchmarking
    let num_writers = 2;
    let num_readers = 3;
    let sizes = [100, 500, 1000];

    for &size in &sizes {
        let num_items = size * 10; // Relative to the bit array size

        // Benchmark for normal BloomFilter
        c.bench_function(&format!("normal_bloom_filter_{}_items", size), |b| {
            let mut bloom = BloomFilter::new(size, 3); // Adjust hash count as needed

            b.iter(|| {
                for i in 0..num_items {
                    let item = format!("item_{}", i);
                    bloom.set(&item);
                    bloom.test(&item); // Test if it's in the BloomFilter
                }
            });
        });

        // Benchmark for thread-safe BloomFilter
        c.bench_function(&format!("thread_safe_bloom_filter_{}_items", size), |b| {
            let bloom = Arc::new(ThreadSafeBF::new(size, 3));

            b.iter(|| {
                let mut handles = Vec::new();

                // Spawn writer threads
                for _ in 0..num_writers {
                    let bloom_clone = Arc::clone(&bloom);
                    let handle = thread::spawn(move || {
                        for i in 0..num_items {
                            let item = format!("item_writer_{}", i);
                            bloom_clone.set(&item).unwrap();
                        }
                    });
                    handles.push(handle);
                }

                // Spawn reader threads
                for _ in 0..num_readers {
                    let bloom_clone = Arc::clone(&bloom);
                    let handle = thread::spawn(move || {
                        for i in 0..num_items {
                            let item = format!("item_writer_{}", i);
                            bloom_clone.test(&item);
                        }
                    });
                    handles.push(handle);
                }

                // Wait for all threads to complete
                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });
        c.bench_function(&format!("atomic_bloom_filter_{}_items", size), |b| {
            let bloom = Arc::new(AtomicBloomFilter::new(size, 3));

            b.iter(|| {
                let mut handles = Vec::new();

                // Spawn writer threads for AtomicBloomFilter
                for _ in 0..num_writers {
                    let bloom_clone = Arc::clone(&bloom);
                    let handle = thread::spawn(move || {
                        for i in 0..num_items {
                            let item = format!("item_writer_{}", i);
                            bloom_clone.set(&item);
                        }
                    });
                    handles.push(handle);
                }

                // Spawn reader threads for AtomicBloomFilter
                for _ in 0..num_readers {
                    let bloom_clone = Arc::clone(&bloom);
                    let handle = thread::spawn(move || {
                        for i in 0..num_items {
                            let item = format!("item_writer_{}", i);
                            bloom_clone.test(&item);
                        }
                    });
                    handles.push(handle);
                }

                // Wait for all threads to complete
                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });
    }
}

criterion_group!(benches, bench_bloom_filter);
criterion_main!(benches);
