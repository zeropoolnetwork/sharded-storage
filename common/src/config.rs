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