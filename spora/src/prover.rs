//! Prover for SPoRa. This prover is optimized for Mersenne31 extension field sealing
//! 
//! We want to prove that the stored vector is a valid random linear combination of payload vector and sealing vector
//! 
//! Sealing vector is stored as Fragment (what have multiple Clusters inside) and merkelized.
//! 
//! Payload vector is stored as Cluster and FRI committed with LDE.
//! 
//! Stored vector should apply the following equation:
//! 
//! Stored_Cluster = Payload_Cluster * Hash(Payload_Cluster_FRI_Commitment, Sealing_Fragment_Merkle_Root) + Sealing_Cluster
//! 
//! This structure allow us easy overwritting the separate Clusters without recomputing the whole stored Fragment.
//! 
//! Here is an issue: The data type is Mersenne31, but RLC over Mersenne31 is unsafe.
//! That's why we chunk the data into 4-order extension field and apply RLC over it.
//! 
//! 


use primitives::{array_to_quadval, quadval_to_array, Challenge, Hash, Poseidon2Challenger, Poseidon2Pcs, QuadVal, Val, POSEIDON2_PCS, poseidon2_perm, POSEIDON2_MMCS};
use primitives::{CLUSTER_SIZE, FRAGMENT_SIZE};
use p3_field::{AbstractExtensionField, Field};
use p3_commit::{Pcs, Mmcs};
use p3_matrix::dense::RowMajorMatrix;
use alloc::vec::Vec;
use alloc::vec;


type MerkleProof = Vec<[Val; 8]>;
type FriProof = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::Proof;



pub struct SporaProof {
    /// FRI proof for the source cluster opening
    pub(crate) source_and_sealed_cluster_proof: FriProof,
    /// Merkle proof for the sealing cluster opening
    pub(crate) sealing_cluster_proof: MerkleProof,
    /// Merkle proof for the fragment opening
    pub(crate) fragment_proof: MerkleProof,
    /// Index of cluster within fragment
    pub(crate) cluster_in_fragment_index: usize,
    /// Index of quadruple within cluster
    pub(crate) quad_in_cluster_index: usize,
}

pub struct SporaOpen {
    pub(crate) sealed_cluster_opening: QuadVal,
    pub(crate) sealing_cluster_opening: QuadVal,
}

pub fn prove_spora_cluster(
    sealed_r: QuadVal,
    sealed_cluster_data: &[Val],
    sealing_fragment_cluster_hashes: &[Hash],
    sealing_cluster_data: &[QuadVal],
    cluster_in_fragment_index: usize,
    quad_in_cluster_index: usize,
) -> (SporaOpen, SporaProof) {
    // TODO: Derive magic numbers from types
    
    assert!(sealed_cluster_data.len() as u64 == CLUSTER_SIZE, "Sealed cluster data length should be equal to cluster size");
    assert!(sealing_cluster_data.len() as u64 * 4 == CLUSTER_SIZE, "Sealing cluster data length should be equal to cluster size");
    assert!(sealing_fragment_cluster_hashes.len() as u64*CLUSTER_SIZE == FRAGMENT_SIZE, "Wrong number of clusters in fragment");
    
    let inverse_r = sealed_r.inverse();

    let source_cluster_data = (0..CLUSTER_SIZE as usize/4).flat_map(|i| {
        let sealed_cluster_quad = array_to_quadval(sealed_cluster_data[i*4..i*4+4].try_into().unwrap());
        let sealing_cluster_quad = sealing_cluster_data[i];
        let source_cluster_quad = (sealed_cluster_quad - sealing_cluster_quad) * inverse_r;
        quadval_to_array(source_cluster_quad)
    }).collect::<Vec<_>>();


    let sealing_cluster_opening = sealing_cluster_data[quad_in_cluster_index];
    let sealed_cluster_opening = array_to_quadval(sealed_cluster_data[quad_in_cluster_index*4..quad_in_cluster_index*4+4].try_into().unwrap());

    let source_cluster_column = RowMajorMatrix::new_col(source_cluster_data);

    let domain = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::natural_domain_for_degree(&POSEIDON2_PCS, CLUSTER_SIZE as usize);


    // TODO: Optimize Plonky3 for single column matrices
    let (_, source_cluster_prover_data) = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::commit(&POSEIDON2_PCS, vec![(domain, source_cluster_column)]);

    // compute sealed_cluster_proof

    let sealed_cluster_column = RowMajorMatrix::new_col(sealed_cluster_data.to_vec());
    let (_, sealed_cluster_prover_data) = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::commit(&POSEIDON2_PCS, vec![(domain, sealed_cluster_column)]);


    let opening_points = (quad_in_cluster_index*4..quad_in_cluster_index*4+4).map(|i| {
        let point = domain.nth_point(i);
        let point_projective = point.to_projective_line().unwrap();
        Challenge::from_base(point_projective)
    }).collect::<Vec<_>>();

    let mut challenger = Poseidon2Challenger::new(poseidon2_perm());

    let (_, source_and_sealed_cluster_proof) = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::open(
        &POSEIDON2_PCS, 
        vec![
            (&source_cluster_prover_data, vec![opening_points.clone()]),
            (&sealed_cluster_prover_data, vec![opening_points])
        ], &mut challenger);


    let sealing_cluster_matrix_data = sealing_cluster_data.iter().copied().flat_map(quadval_to_array).collect::<Vec<_>>();
    let sealing_cluster_matrix = RowMajorMatrix::new(sealing_cluster_matrix_data, 4);

    let (sealing_cluster_commit, sealing_cluster_prover_data) = POSEIDON2_MMCS.commit_matrix(sealing_cluster_matrix);

    let (_, sealing_cluster_proof) = POSEIDON2_MMCS.open_batch(quad_in_cluster_index, &sealing_cluster_prover_data);

    assert!(sealing_fragment_cluster_hashes[cluster_in_fragment_index] == sealing_cluster_commit, "Sealing fragment cluster hash should be equal to commit");

    let sealing_fragment_matrix_data = sealing_fragment_cluster_hashes.iter().copied().flatten().collect::<Vec<_>>();
    let sealing_fragment_matrix = RowMajorMatrix::new(sealing_fragment_matrix_data, 8);

    let (_, fragment_prover_data) = POSEIDON2_MMCS.commit_matrix(sealing_fragment_matrix);

    let (_, fragment_proof) = POSEIDON2_MMCS.open_batch(cluster_in_fragment_index, &fragment_prover_data);

    
    

    // TODO: assert, that sealed_rlc is derived correctly


    // Opening data is (sealed_cluster_opening, sealing_cluster_opening)
    // Proof is composite of opening quadval, merkle proofs, fri proof, cluster_in_fragment_index and quad_in_cluster_index


    (
        SporaOpen {
            sealed_cluster_opening,
            sealing_cluster_opening,
        },
        SporaProof {
            source_and_sealed_cluster_proof,
            sealing_cluster_proof,
            fragment_proof,
            cluster_in_fragment_index,
            quad_in_cluster_index,
        }
    )
}