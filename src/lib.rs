use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

use sha2::{Digest, Sha256};

pub struct BloomFilter {
    bit_array: Vec<bool>,
    num_hashes: usize,
    size: usize,
    //hash_funcs: Vec<Box<dyn Fn(&[u8]) -> u64>>,
}

pub struct ThreadSafeBF {
    bf: Arc<RwLock<BloomFilter>>,
}

pub struct AtomicBloomFilter {
    bit_array: Vec<AtomicBool>,
    num_hashes: usize,
    size: usize,
}

impl AtomicBloomFilter {
    pub fn new(
        size: usize,
        num_hashes: usize, //hash_funcs: Vec<Box<dyn Fn(&[u8]) -> u64>>
    ) -> Self {
        AtomicBloomFilter {
            bit_array: (0..size).map(|_| AtomicBool::new(false)).collect(),
            num_hashes,
            size,
            //       hash_funcs,
        }
    }
    fn hash(&self, item: &str, i: usize) -> usize {
        let mut hasher = Sha256::new();
        hasher.update(item.as_bytes());
        hasher.update(i.to_le_bytes());
        let hash_res = hasher.finalize();

        let mut hash_val = [0u8; 8];
        hash_val.copy_from_slice(&hash_res[0..8]); // Take the first 8 bytes of the hash
        usize::from_le_bytes(hash_val) % self.size
    }

    pub fn set(&self, item: &str) {
        for i in 0..self.num_hashes {
            let idx: usize = self.hash(&item, i);
            self.bit_array[idx].store(true, Ordering::Relaxed);
        }
    }

    pub fn test(&self, item: &str) -> bool {
        for i in 0..self.num_hashes {
            let idx: usize = self.hash(item, i);
            if !self.bit_array[idx].load(Ordering::Relaxed) {
                return false;
            }
        }
        true
    }
}

impl BloomFilter {
    pub fn new(
        size: usize,
        num_hashes: usize, //hash_funcs: Vec<Box<dyn Fn(&[u8]) -> u64>>
    ) -> Self {
        BloomFilter {
            bit_array: vec![false; size],
            num_hashes,
            size,
            //       hash_funcs,
        }
    }

    // Creating Multiple Hashes with one hash function
    fn hash(&self, item: &str, i: usize) -> usize {
        // Convert the first 8 bytes of the hash to a usize and modulo it by the bit array size
        // Ex. for "foo"
        // 1. SHA256("foo") = X
        // 2. i = 0 as byte -> [0,0,0,0,0,0,0,0]
        // 3. SHA256("foo" + [0,0,0,0,0,0,0,0]) = e02aa5a0b4e8a3644f8e9c10459dfb64609c95c91fe49328d228f3f10636c2ec
        // 4. Take first 8 bytes: e02aa5a0b4e8a364 as byte -> [224, 42, 165, 160, 180, 232, 163, 100]
        // 5. usize::from_le_bytes([224, 42, 165, 160, 180, 232, 163, 100]) = 7235236067926870112
        // 6. return 7235236067926870112 % 1000 = 112

        let mut hasher = Sha256::new();
        hasher.update(item.as_bytes());
        hasher.update(i.to_le_bytes());
        let hash_res = hasher.finalize();

        let mut hash_val = [0u8; 8];
        hash_val.copy_from_slice(&hash_res[0..8]); // Take the first 8 bytes of the hash
        usize::from_le_bytes(hash_val) % self.size
    }

    pub fn set(&mut self, item: &str) {
        for i in 0..self.num_hashes {
            let idx: usize = self.hash(&item, i);
            self.bit_array[idx] = true;
        }
    }

    pub fn test(&self, item: &str) -> bool {
        for i in 0..self.num_hashes {
            let idx: usize = self.hash(item, i);
            if !self.bit_array[idx] {
                return false;
            }
        }
        true
    }

    //For setting hash functions beside SHA256 by user
    pub fn set_hash_fn(&mut self, hashFn: Vec<Box<dyn Fn(&[u8]) -> u64>>) {}
    pub fn reset(&mut self) {
        self.bit_array.fill(false);
    }
}

impl ThreadSafeBF {
    pub fn new(size: usize, num_hashes: usize) -> Self {
        Self {
            bf: Arc::new(RwLock::new(BloomFilter::new(size, num_hashes))),
        }
    }
    pub fn set(&self, item: &str) -> Result<(), String> {
        match self.bf.write() {
            Ok(mut blooom) => {
                blooom.set(item);
                Ok(())
            }
            Err(_) => Err("Failed to acquire write lock on BloomFilter. Lock is poisoned.".into()),
        }
    }

    pub fn test(&self, item: &str) -> bool {
        let bloom = self.bf.read().unwrap();
        bloom.test(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_test() {
        let mut bloom = BloomFilter::new(100, 3);

        bloom.set("foo");
        bloom.set("bar");

        assert!(bloom.test("foo"));
        assert!(bloom.test("bar"));
        assert!(!bloom.test("baz")); // "baz" should not be in the filter
    }

    #[test]
    fn test_false_positive() {
        let mut bloom = BloomFilter::new(10, 2);

        bloom.set("apple");
        bloom.set("orange");

        assert!(bloom.test("apple"));
        assert!(bloom.test("orange"));
        // Due to the small size, "grape" might cause a false positive
        assert!(!bloom.test("grape"));
    }

    #[test]
    fn test_concurrent_reads_and_writes() {
        let bloom = Arc::new(ThreadSafeBF::new(1000, 5));

        let bloom_clone1 = Arc::clone(&bloom);
        let bloom_clone2 = Arc::clone(&bloom);
        let bloom_clone3 = Arc::clone(&bloom);
        let bloom_clone4 = Arc::clone(&bloom);
        let bloom_clone5 = Arc::clone(&bloom);

        let writer1 = thread::spawn(move || {
            bloom_clone1.set("concurrent_item_1");
            bloom_clone1.set("concurrent_item_2");
        });

        let writer2 = thread::spawn(move || {
            bloom_clone4.set("concurrent_item_3");
            bloom_clone4.set("concurrent_item_4");
        });

        let reader1 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(10));
            assert!(bloom_clone2.test("concurrent_item_1"));
            assert!(bloom_clone2.test("concurrent_item_2"));
            assert!(!bloom_clone2.test("non_existent_item"));
        });

        let reader2 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(20));
            assert!(bloom_clone3.test("concurrent_item_3"));
            assert!(bloom_clone3.test("concurrent_item_4"));
        });

        let reader3 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(15));
            assert!(bloom_clone5.test("concurrent_item_1"));
            assert!(!bloom_clone5.test("non_existent_item2"));
        });

        writer1.join().unwrap();
        writer2.join().unwrap();
        reader1.join().unwrap();
        reader2.join().unwrap();
        reader3.join().unwrap();
    }

    #[test]
    fn test_concurrent_reads_and_writes_atomic() {
        let bloom = Arc::new(AtomicBloomFilter::new(1000, 5));

        let bloom_clone1 = Arc::clone(&bloom);
        let bloom_clone2 = Arc::clone(&bloom);
        let bloom_clone3 = Arc::clone(&bloom);
        let bloom_clone4 = Arc::clone(&bloom);
        let bloom_clone5 = Arc::clone(&bloom);

        // Writer thread 1
        let writer1 = thread::spawn(move || {
            bloom_clone1.set("concurrent_item_1");
            bloom_clone1.set("concurrent_item_2");
        });

        // Writer thread 2
        let writer2 = thread::spawn(move || {
            bloom_clone4.set("concurrent_item_3");
            bloom_clone4.set("concurrent_item_4");
        });

        // Reader thread 1
        let reader1 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(10)); // Sleep to allow writers to run first
            assert!(bloom_clone2.test("concurrent_item_1"));
            assert!(bloom_clone2.test("concurrent_item_2"));
            assert!(!bloom_clone2.test("non_existent_item"));
        });

        // Reader thread 2
        let reader2 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(20)); // Sleep to allow more time for writers
            assert!(bloom_clone3.test("concurrent_item_3"));
            assert!(bloom_clone3.test("concurrent_item_4"));
        });

        // Reader thread 3
        let reader3 = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(15)); // A different sleep time
            assert!(bloom_clone5.test("concurrent_item_1"));
            assert!(!bloom_clone5.test("non_existent_item2"));
        });

        // Join all threads to ensure the test completes
        writer1.join().unwrap();
        writer2.join().unwrap();
        reader1.join().unwrap();
        reader2.join().unwrap();
        reader3.join().unwrap();
    }
}
