use p3_mersenne_31::Mersenne31;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    /// columns
    pub n: usize,
    /// rows
    pub m: usize,
    /// rows after blowup
    pub q: usize,
    /// number of sectors
    pub k: usize,
}

impl StorageConfig {
    pub fn dev() -> Self {
        Self {
            n: 16384,
            m: 8,
            q: 32,
            k: 2097152,
        }
    }

    pub fn prod() -> Self {
        Self {
            n: 16384,
            m: 64,
            q: 512,
            k: 33554432,
        }
    }

    pub fn num_chunks(&self) -> usize {
        self.q / self.m
    }

    pub fn sector_capacity(&self) -> usize {
        self.n * self.m
    }

    pub fn sector_capacity_bytes(&self) -> usize {
        self.n * self.m * 30 / 8
    }
}
