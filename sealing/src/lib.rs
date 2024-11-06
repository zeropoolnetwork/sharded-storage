#![no_std]

extern crate alloc;



use p3_circle::{CircleDomain,CircleEvaluations};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use itertools::Itertools;
use alloc::vec::Vec;


use primitives::{POSEIDON2_PERM, M31StreamCipher, Val, LOG_FRAGMENT_SIZE, FRAGMENT_SIZE, Hash, poseidon2_hash_slice};


pub fn sealing_vec(seed: Hash) -> Vec<Val> {
    let stream = M31StreamCipher::new(POSEIDON2_PERM.clone());
    let data = stream.cipher(seed.as_ref()).take(FRAGMENT_SIZE as usize).collect_vec();
    let data_domain = CircleDomain::<Val>::standard(LOG_FRAGMENT_SIZE);
    let data_coeffs = RowMajorMatrix::new(data, 1);
    let data_evals = CircleEvaluations::evaluate(data_domain, data_coeffs).to_natural_order().to_row_major_matrix();
    data_evals.values
}

pub fn get_fragment_seed(node_id:usize, volume_id:usize, segment_id:usize, fragment_id:usize) -> Hash {
    let preimage = [Val::new(node_id as u32), Val::new(fragment_id as u32), Val::new(segment_id as u32), Val::new(volume_id as u32)];
    poseidon2_hash_slice(&preimage)
}

