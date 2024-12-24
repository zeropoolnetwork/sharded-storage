use primitives::{array_to_quadval, quadval_to_array, Challenge, Hash, Poseidon2Challenger, Poseidon2Pcs, QuadVal, Val, POSEIDON2_PCS, poseidon2_perm};
use primitives::CLUSTER_SIZE;
use p3_commit::Pcs;
use p3_challenger::{CanObserve, CanSample};
use p3_matrix::dense::RowMajorMatrix;
use alloc::vec::Vec;
use alloc::vec;


pub fn rlc(source_cluster: &[Val], sealing_cluster: &[QuadVal], sealing_fragment_root: Hash) -> (QuadVal, Vec<Val> ) {
    assert!(source_cluster.len() as u64 == CLUSTER_SIZE, "Payload cluster size should be equal to cluster size");
    assert!(sealing_cluster.len() as u64 * 4 == CLUSTER_SIZE, "Sealing cluster size should be equal to cluster size");


    let domain = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::natural_domain_for_degree(&POSEIDON2_PCS, CLUSTER_SIZE as usize);
    let source_cluster_column = RowMajorMatrix::new_col(source_cluster.to_vec());
    let (commit, _) = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::commit(&POSEIDON2_PCS, vec![(domain, source_cluster_column)]);
    
    let mut challenger = Poseidon2Challenger::new(poseidon2_perm());

    challenger.observe(commit);
    challenger.observe(sealing_fragment_root);

    let r = array_to_quadval(challenger.sample_array::<4>());

    let sealed_cluster = (0..CLUSTER_SIZE as usize/4).flat_map(|i| {
        let source_cluster_quad = array_to_quadval(source_cluster[i*4..i*4+4].try_into().unwrap());
        let sealing_cluster_quad = sealing_cluster[i];
        let sealed_cluster_quad = source_cluster_quad * r + sealing_cluster_quad;
        quadval_to_array(sealed_cluster_quad)
    }).collect::<Vec<_>>();

    (r, sealed_cluster)
}