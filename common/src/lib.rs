use p3_field::AbstractField;
use p3_mersenne_31::Mersenne31;

use crate::config::StorageConfig;

pub mod config;
pub mod contract;
pub mod encode;

pub type Field = Mersenne31;

/// Mock
pub fn reconstruct(data: &[Mersenne31], config: &StorageConfig) -> Vec<Mersenne31> {
    let factor = config.q / config.m;
    let mut result = Vec::with_capacity(data.len() / factor);

    for chunk in data.chunks(factor) {
        result.push(chunk[0]); // FIXME
    }

    result
}
