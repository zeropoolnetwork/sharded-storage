use primitives::{POSEIDON2_HASH, Val};

use p3_symmetric::CryptographicHasher;

pub trait UnstructuredStorageReader :Send + Sync {
    fn read(&self, index: u128) -> Val;
    fn log_len(&self) -> usize;
}

pub struct SimpleTestingStorageEmulator {
    log_size: usize,
}

impl SimpleTestingStorageEmulator {
    pub fn new(log_size: usize) -> Self {
        Self { log_size }
    }
}

impl UnstructuredStorageReader for SimpleTestingStorageEmulator {
    fn read(&self, index: u128) -> Val {
        assert!(index < (1 << self.log_size));
        const BITS_PER_SAMPLE : usize = 16;
        const MASK : u32 = (1 << BITS_PER_SAMPLE) - 1;
        let parts = [
            Val::new(index as u32 & MASK),
            Val::new((index >> BITS_PER_SAMPLE) as u32 & MASK),
            Val::new((index >> (2 * BITS_PER_SAMPLE)) as u32 & MASK),
            Val::new((index >> (3 * BITS_PER_SAMPLE)) as u32 & MASK),
            Val::new((index >> (4 * BITS_PER_SAMPLE)) as u32 & MASK),
            Val::new((index >> (5 * BITS_PER_SAMPLE)) as u32 & MASK),
            Val::new((index >> (6 * BITS_PER_SAMPLE)) as u32 & MASK),
            Val::new((index >> (7 * BITS_PER_SAMPLE)) as u32 & MASK),
        ];

        POSEIDON2_HASH.hash_iter(parts)[0]
    }

    fn log_len(&self) -> usize {
        self.log_size
    }
}
