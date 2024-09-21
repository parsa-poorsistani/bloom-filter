use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

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
    pub fn add(&self, item: &str) {
        let mut bloom = self.bf.try_write().unwrap();
        bloom.set(item);
    }

    pub fn test(&self, item: &str) -> bool {
        let bloom = self.bf.try_read().unwrap();
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
}
