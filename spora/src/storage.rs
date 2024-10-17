use p3_mersenne_31::Mersenne31;

use primitives::poseidon2_hash;

pub trait UnstructuredStorageReader :Send + Sync {
    fn read(&self, index: u128) -> Mersenne31;
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
    fn read(&self, index: u128) -> Mersenne31 {
        assert!(index < (1 << self.log_size));
        const BITS_PER_SAMPLE : usize = 16;
        const MASK : u32 = (1 << BITS_PER_SAMPLE) - 1;
        let parts = [
            Mersenne31::new(index as u32 & MASK),
            Mersenne31::new((index >> BITS_PER_SAMPLE) as u32 & MASK),
            Mersenne31::new((index >> (2 * BITS_PER_SAMPLE)) as u32 & MASK),
            Mersenne31::new((index >> (3 * BITS_PER_SAMPLE)) as u32 & MASK),
            Mersenne31::new((index >> (4 * BITS_PER_SAMPLE)) as u32 & MASK),
            Mersenne31::new((index >> (5 * BITS_PER_SAMPLE)) as u32 & MASK),
            Mersenne31::new((index >> (6 * BITS_PER_SAMPLE)) as u32 & MASK),
            Mersenne31::new((index >> (7 * BITS_PER_SAMPLE)) as u32 & MASK),
        ];

        poseidon2_hash(&parts)[0]
    }

    fn log_len(&self) -> usize {
        self.log_size
    }
}
