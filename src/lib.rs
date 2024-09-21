pub struct BloomFilter {
    bit_array: Vec<bool>,
    num_hashes: usize,
    size: usize,
}

impl BloomFilter {
    pub fn new(size: usize, num_hashes: usize) -> Self {
        BloomFilter {
            bit_array: vec![false; size],
            num_hashes,
            size,
        }
    }

    fn hash(&self, item: &str, i: usize) -> usize {
        return 1;
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
}
