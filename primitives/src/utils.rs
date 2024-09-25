use p3_mersenne_31::Mersenne31;
use p3_symmetric::CryptographicHasher;
use p3_circle::CircleDomain;
use p3_commit::Pcs;
use p3_matrix::dense::RowMajorMatrix;

use alloc::vec::Vec;

use crate::config::{POSEIDON2_M31_HASH, Challenge, Poseidon2M31Challenger, Poseidon2M31Pcs,pcs_config};

pub fn poseidon2_hash(input: Vec<Mersenne31>) -> [Mersenne31; 8] {
    POSEIDON2_M31_HASH.hash_iter(input)
}

type PcsCommitment = <Poseidon2M31Pcs as Pcs::<Challenge,Poseidon2M31Challenger>>::Commitment;
type PcsProverData = <Poseidon2M31Pcs as Pcs::<Challenge,Poseidon2M31Challenger>>::ProverData;


pub fn pcs_commit(data: Vec<(CircleDomain<Mersenne31>,RowMajorMatrix<Mersenne31>)>) -> (PcsCommitment, PcsProverData)  {
    Pcs::<Challenge,Poseidon2M31Challenger>::commit(&pcs_config(), data)
}