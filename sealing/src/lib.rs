#![no_std]

extern crate alloc;



use p3_circle::{CircleDomain,CircleEvaluations};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use itertools::Itertools;

use alloc::vec::Vec;


use primitives::{POSEIDON2_PERM, M31StreamCipher, Val};

pub fn sealing_vec(seed: [Val;8], log_fragment_shard_size:usize) -> Vec<Val> {
    let stream = M31StreamCipher::new(POSEIDON2_PERM.clone());
    let data = stream.cipher(&seed).take(1<<log_fragment_shard_size).collect_vec();
    let data_domain = CircleDomain::<Val>::standard(log_fragment_shard_size);
    let data_coeffs = RowMajorMatrix::new(data, 1);
    let data_evals = CircleEvaluations::evaluate(data_domain, data_coeffs).to_natural_order().to_row_major_matrix();
    data_evals.values
}