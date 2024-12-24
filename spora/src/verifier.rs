use primitives::{array_to_quadval, quadval_to_array, Challenge, Hash, Poseidon2Challenger, Poseidon2Pcs, POSEIDON2_PCS, poseidon2_perm, POSEIDON2_MMCS};
use primitives::{CLUSTER_SIZE, FRAGMENT_SIZE};
use p3_field::{AbstractExtensionField, Field};
use p3_commit::{Pcs, Mmcs};
use p3_challenger::{CanObserve, CanSample};
use p3_matrix::Dimensions;
use alloc::vec::Vec;
use alloc::vec;
use itertools::izip;
use crate::{SporaProof, SporaOpen};

pub fn verify_spora_cluster(open: SporaOpen, proof: SporaProof, source_cluster_commit: Hash,sealed_cluster_commit: Hash, sealing_fragment_root: Hash, sealing_cluster_root: Hash) -> bool {
    let SporaOpen {
        sealed_cluster_opening,
        sealing_cluster_opening,
    } = open;
    
    let SporaProof {
        source_and_sealed_cluster_proof,
        sealing_cluster_proof,
        fragment_proof,
        cluster_in_fragment_index,
        quad_in_cluster_index,
    } = proof;

    let r = {
        let mut challenger = Poseidon2Challenger::new(poseidon2_perm());

        challenger.observe(sealed_cluster_commit);
        challenger.observe(sealing_fragment_root);

        array_to_quadval(challenger.sample_array::<4>())
    };

    let source_cluster_opening = (sealed_cluster_opening - sealing_cluster_opening) * r.inverse();

    let domain = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::natural_domain_for_degree(&POSEIDON2_PCS, CLUSTER_SIZE as usize);

    let opening_points = (quad_in_cluster_index*4..quad_in_cluster_index*4+4).map(|i| {
        let point = domain.nth_point(i);
        let point_projective = point.to_projective_line().unwrap();
        Challenge::from_base(point_projective)
    }).collect::<Vec<_>>();

    let source_opening_points_with_values = izip!(opening_points.clone(), quadval_to_array(source_cluster_opening)).map(|(point, value)| (point, vec![Challenge::from_base(value)])).collect::<Vec<_>>();
    let sealed_opening_points_with_values = izip!(opening_points, quadval_to_array(sealed_cluster_opening)).map(|(point, value)| (point, vec![Challenge::from_base(value)])).collect::<Vec<_>>();

    let mut challenger = Poseidon2Challenger::new(poseidon2_perm());

    // verify FRI opening
    let source_and_sealed_cluster_proof_result = <Poseidon2Pcs as Pcs<Challenge, Poseidon2Challenger>>::verify(&POSEIDON2_PCS, 
        vec![(
            source_cluster_commit, 
            vec![
                (domain, source_opening_points_with_values),
                (domain, sealed_opening_points_with_values)
            ]
        )], 
        &source_and_sealed_cluster_proof, &mut challenger);
    
    // if result is Err, return false
    if source_and_sealed_cluster_proof_result.is_err() {
        return false;
    }


    let sealing_cluster_dimensions = vec![Dimensions {width: 1, height: CLUSTER_SIZE as usize/4}];
    
    let sealing_cluster_proof_result = POSEIDON2_MMCS.verify_batch(&sealing_cluster_root, &sealing_cluster_dimensions, quad_in_cluster_index, &[quadval_to_array(sealing_cluster_opening).into()], &sealing_cluster_proof);

    if sealing_cluster_proof_result.is_err() {
        return false;
    }

    let fragment_dimensions = vec![Dimensions {width: 1, height: (FRAGMENT_SIZE / CLUSTER_SIZE) as usize}];

    let fragment_proof_result = POSEIDON2_MMCS.verify_batch(&sealing_fragment_root, &fragment_dimensions, cluster_in_fragment_index, &[sealing_cluster_root.as_ref().into()], &fragment_proof);

    if fragment_proof_result.is_err() {
        return false;
    }

    true
}

