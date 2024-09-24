
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


use core::marker::PhantomData;

pub type Poseidon2M31Perm = Poseidon2<Mersenne31, Poseidon2ExternalMatrixGeneral, DiffusionMatrixMersenne31, 16, 5>;
pub type Poseidon2M31Hash = PaddingFreeSponge<Poseidon2M31Perm, 16, 8, 8>;
pub type Poseidon2M31Compress = TruncatedPermutation<Poseidon2M31Perm, 2, 8, 16>;
pub type Poseidon2M31Mmcs = FieldMerkleTreeMmcs<
    <Mersenne31 as Field>::Packing,
    <Mersenne31 as Field>::Packing,
    Poseidon2M31Hash,
    Poseidon2M31Compress,
    8,
>;

pub type Challenge = BinomialExtensionField<Mersenne31, 3>;
pub type Poseidon2M31ChallengeMmcs = ExtensionMmcs<Mersenne31, Challenge, Poseidon2M31Mmcs>;
pub type Poseidon2M31Challenger = DuplexChallenger<Mersenne31, Poseidon2M31Perm, 16, 8>;

pub type Poseidon2M31Pcs = CirclePcs<Mersenne31, Poseidon2M31Mmcs, Poseidon2M31ChallengeMmcs>;
pub type Poseidon2M31StarkConfig = StarkConfig<Poseidon2M31Pcs, Challenge, Poseidon2M31Challenger>;

pub fn poseidon2_m31_config() -> Poseidon2M31Perm {
    Poseidon2M31Perm::new(
        crate::POSEIDON2_M31_W16_D5_ROUNDS_F,
        crate::POSEIDON2_M31_W16_D5_EXTERNAL_CONSTANTS.to_vec(),
        Poseidon2ExternalMatrixGeneral,
        crate::POSEIDON2_M31_W16_D5_ROUNDS_P,
        crate::POSEIDON2_M31_W16_D5_INTERNAL_CONSTANTS.to_vec(),
        DiffusionMatrixMersenne31,
        
    )
}

pub fn poseidon2_m31_hash() -> Poseidon2M31Hash {
    Poseidon2M31Hash::new(poseidon2_m31_config())
}

pub fn poseidon2_m31_compress() -> Poseidon2M31Compress {
    Poseidon2M31Compress::new(poseidon2_m31_config())
}

pub fn poseidon2_m31_mmcs() -> Poseidon2M31Mmcs {
    Poseidon2M31Mmcs::new(poseidon2_m31_hash(), poseidon2_m31_compress())
}

pub fn poseidon2_m31_challenge_mmcs() -> Poseidon2M31ChallengeMmcs {
    Poseidon2M31ChallengeMmcs::new(poseidon2_m31_mmcs())
}

pub fn fri_config() -> FriConfig<Poseidon2M31ChallengeMmcs> {
    FriConfig {
        log_blowup:1,
        num_queries:100,
        proof_of_work_bits:16,
        mmcs: poseidon2_m31_challenge_mmcs()
    }
}

pub fn pcs_config() -> Poseidon2M31Pcs {
    Poseidon2M31Pcs {
        mmcs: poseidon2_m31_mmcs(),
        fri_config: fri_config(),
        _phantom: PhantomData
    }
}

pub fn stark_config() -> Poseidon2M31StarkConfig {
    Poseidon2M31StarkConfig::new(pcs_config())
}

