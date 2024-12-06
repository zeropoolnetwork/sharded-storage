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
    pub fn prod() -> Self {
        Self {
            n: 16384,
            m: 64,
            q: 512,
            k: 33554432,
        }
    }

    pub fn dev() -> Self {
        Self {
            n: 65536,
            m: 4,
            q: 16,
            k: 2097152, // FIXME: incorrect, but doesn't matter for now
        }
    }

    pub fn num_clusters(&self) -> usize {
        self.q
    }

    pub fn cluster_size(&self) -> usize {
        self.n * self.m
    }

    pub fn cluster_size_bytes(&self) -> usize {
        self.n * self.m * 30 / 8
    }

    pub fn log_blowup_factor(&self) -> usize {
        (self.q / self.m).ilog2() as usize
    }
}
