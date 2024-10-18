
use p3_mersenne_31::{DiffusionMatrixMersenne31, Mersenne31};
use p3_symmetric::{PaddingFreeSponge, TruncatedPermutation};
use p3_merkle_tree::FieldMerkleTreeMmcs;

use p3_poseidon2::{Poseidon2, Poseidon2ExternalMatrixGeneral};
use p3_commit::ExtensionMmcs;
use p3_field::Field;
use p3_field::extension::BinomialExtensionField;
use p3_challenger::DuplexChallenger;

use p3_uni_stark::StarkConfig;
use p3_circle::CirclePcs;
use p3_fri::FriConfig;
use lazy_static::lazy_static;

use core::marker::PhantomData;

use crate::utils::StreamCipher;

pub type Val = Mersenne31;

pub type Poseidon2Perm = Poseidon2<Val, Poseidon2ExternalMatrixGeneral, DiffusionMatrixMersenne31, 16, 5>;
pub type Poseidon2Hash = PaddingFreeSponge<Poseidon2Perm, 16, 8, 8>;
pub type Poseidon2Compress = TruncatedPermutation<Poseidon2Perm, 2, 8, 16>;
pub type Poseidon2Mmcs = FieldMerkleTreeMmcs<
    <Val as Field>::Packing,
    <Val as Field>::Packing,
    Poseidon2Hash,
    Poseidon2Compress,
    8,
>;

pub type Challenge = BinomialExtensionField<Val, 3>;
pub type Poseidon2ChallengeMmcs = ExtensionMmcs<Val, Challenge, Poseidon2Mmcs>;
pub type Poseidon2Challenger = DuplexChallenger<Val, Poseidon2Perm, 16, 8>;

pub type Poseidon2Pcs = CirclePcs<Val, Poseidon2Mmcs, Poseidon2ChallengeMmcs>;
pub type Poseidon2StarkConfig = StarkConfig<Poseidon2Pcs, Challenge, Poseidon2Challenger>;

pub type Hash = p3_symmetric::Hash<Val, Val, 8>;

pub type M31StreamCipher = StreamCipher<Val, Poseidon2Perm, 16, 8>;

lazy_static!{
    pub static ref POSEIDON2_PERM: Poseidon2Perm = poseidon2_perm();
    pub static ref POSEIDON2_HASH: Poseidon2Hash = poseidon2_hash();
    pub static ref POSEIDON2_COMPRESS: Poseidon2Compress = poseidon2_compress();
    pub static ref POSEIDON2_MMCS: Poseidon2Mmcs = poseidon2_mmcs();
    pub static ref POSEIDON2_CHALLENGE_MMCS: Poseidon2ChallengeMmcs = poseidon2_challenge_mmcs();
}



pub fn poseidon2_perm() -> Poseidon2Perm {
    Poseidon2Perm::new(
        crate::POSEIDON2_W16_D5_ROUNDS_F,
        crate::POSEIDON2_W16_D5_EXTERNAL_CONSTANTS.to_vec(),
        Poseidon2ExternalMatrixGeneral,
        crate::POSEIDON2_W16_D5_ROUNDS_P,
        crate::POSEIDON2_W16_D5_INTERNAL_CONSTANTS.to_vec(),
        DiffusionMatrixMersenne31,
        
    )
}


pub fn poseidon2_hash() -> Poseidon2Hash {
    Poseidon2Hash::new(POSEIDON2_PERM.clone())
}



pub fn poseidon2_compress() -> Poseidon2Compress {
    Poseidon2Compress::new(POSEIDON2_PERM.clone())
}

pub fn poseidon2_mmcs() -> Poseidon2Mmcs {
    Poseidon2Mmcs::new(POSEIDON2_HASH.clone(), POSEIDON2_COMPRESS.clone())
}

pub fn poseidon2_challenge_mmcs() -> Poseidon2ChallengeMmcs {
    Poseidon2ChallengeMmcs::new(POSEIDON2_MMCS.clone())
}

pub fn fri_config() -> FriConfig<Poseidon2ChallengeMmcs> {
    FriConfig {
        log_blowup:1,
        num_queries:100,
        proof_of_work_bits:16,
        mmcs: POSEIDON2_CHALLENGE_MMCS.clone()
    }
}

pub fn pcs_config() -> Poseidon2Pcs {
    Poseidon2Pcs {
        mmcs: POSEIDON2_MMCS.clone(),
        fri_config: fri_config(),
        _phantom: PhantomData
    }
}

pub fn stark_config() -> Poseidon2StarkConfig {
    Poseidon2StarkConfig::new(pcs_config())
}

