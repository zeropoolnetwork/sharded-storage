use p3_maybe_rayon::prelude::*;

use primitives::{Poseidon2Challenger, POSEIDON2_PERM, poseidon2_hash_slice};

use p3_field::PrimeField32;
use p3_challenger::{CanObserve, CanSampleBits};

use alloc::vec::Vec;

use itertools::Itertools;


use crate::storage::UnstructuredStorageReader;
use crate::types::Nonce;

#[derive(Clone, Copy, Debug)]
pub struct SPoRAConfig {
    max_nonce: u64,
    log_complexity: usize,
    n_samples: usize,
    log_storage_size: usize
}




// TODO: it is suboptimal. Taking only half of the bits to reduce bias
fn sample_index(challenger: &mut Poseidon2Challenger, log_size:usize) -> u64 {
    assert!(log_size <= 64);
    const BITS_PER_SAMPLE : usize = 16;
    let mut res = 0;
    let mut rem_bits = log_size;

    while rem_bits > 0 {
        let len_bits = rem_bits.min(BITS_PER_SAMPLE);
        let bits = challenger.sample_bits(len_bits);
        res = (res << len_bits) + bits as u64;
        rem_bits -= len_bits;
    }
    res
}




// Return finding complexity
fn spora_with_nonce(config:&SPoRAConfig, nonce:Nonce, storage: &impl UnstructuredStorageReader) -> usize {
    let mut challenger = Poseidon2Challenger::new(POSEIDON2_PERM.clone());

    challenger.observe(nonce.as_mersenne_31_word());

    let values = (0..config.n_samples)
        .map(|_| {
            let index = sample_index(&mut challenger, config.log_storage_size);
            storage.read(index)
        }).collect_vec();

    let hash = poseidon2_hash_slice(&values);


    hash.as_ref()[0].as_canonical_u32().leading_zeros() as usize - 1
}

pub fn spora(config:&SPoRAConfig, storage: &impl UnstructuredStorageReader) -> Vec<(Nonce,usize)> {
    (0..config.max_nonce).into_par_iter().map(|nonce| {
        let nonce = Nonce::new(nonce);
        let complexity = spora_with_nonce(config, nonce, storage);
        (nonce, complexity)
    }).filter(|(_,complexity)| *complexity >= config.log_complexity).collect::<Vec<_>>()
}
        

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SimpleTestingStorageEmulator;
    use libc_print::std_name::println;

    #[test]
    fn test_spora() {
        let config = SPoRAConfig {
            max_nonce: 1024,
            log_complexity: 9,
            n_samples: 10,
            log_storage_size: 30
        };

        let storage = SimpleTestingStorageEmulator::new(config.log_storage_size);

        let result = spora(&config, &storage);

        println!("Result: {:?}", result);

    }
}