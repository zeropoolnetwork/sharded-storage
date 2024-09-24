use alloc::vec;
use alloc::vec::Vec;
use itertools::{Itertools, iterate};
use p3_challenger::{CanObserve, FieldChallenger};
use p3_circle::{CircleDomain, CircleEvaluations, Point};
use p3_commit::Mmcs;
use p3_field::PackedValue;
use p3_matrix::{Matrix, dense::RowMajorMatrix};
use p3_maybe_rayon::prelude::*;
use p3_mersenne_31::Mersenne31;
use p3_util::log2_strict_usize;
use primitives::*;




pub struct OptimisticCorrectableCommitment {
    pub pcs_commitment: Hash,
    pub shards_root_hash: Hash,
    pub opening_at_chi: Vec<Challenge>
}

// Returns subcoset for fast data recovery
//
pub fn subdomain(i:usize, log_blowup: usize, log_n:usize) -> CircleDomain<Mersenne31> {
    assert!(i < (1<<log_blowup));

    let shift0 = Point::generator(log_n + log_blowup + 1);
    let gen0 = Point::generator(log_n + log_blowup);

    CircleDomain::new(log_n, shift0+gen0*i)
}

pub fn subdomain_indexes(i:usize, log_blowup: usize, log_n:usize) -> Vec<usize> {
    let blowup = 1<<log_blowup;
    let n_half = 1<<(log_n-1);


    let subcoset0_indexes = iterate(i, |&i| (i+2*blowup)).take(n_half);
    let subcoset1_indexes = iterate(2*blowup - i - 1, |&i| (i+2*blowup)).take(n_half);
    subcoset0_indexes.interleave(subcoset1_indexes).collect_vec()
}


// Computes all shards and commitment according to https://ethresear.ch/t/using-fri-for-da-with-optimistic-correctable-commitments-in-rollups/20467
//
pub fn commit<M:Matrix<Mersenne31>>(data: M, log_blowup: usize) -> (OptimisticCorrectableCommitment, Vec<Vec<Mersenne31>>)
{

    let mmcs = POSEIDON2_M31_MMCS.clone();

    let data_width = data.width();
    let data_height = data.height();
    let log_data_width = log2_strict_usize(data_width);
    let log_n_shards = log_data_width + log_blowup;
    let log_data_height = log2_strict_usize(data_height);


    let data_domain = CircleDomain::<Mersenne31>::standard(log_data_width);
    let shards_domain = CircleDomain::<Mersenne31>::standard(log_n_shards);
    let commitment_domain = CircleDomain::<Mersenne31>::standard(log_data_height);
    let data = data.to_row_major_matrix();
    let transposed_data = data.transpose();


    // OPTIMIZATION TIPS:
    // 1. replace source representation from evaluations to coefficients (for sharding only, for commitment use evaluations)
    // 2. store result in cfft order
    
    let blown_up_data = CircleEvaluations::from_natural_order(data_domain, transposed_data).extrapolate(shards_domain).to_natural_order();

    let shards = blown_up_data.par_rows().map(|row| {
        row.collect_vec()
    }).collect::<Vec<_>>();

    let shards_commitments = shards.iter().map(|row| {
        mmcs.commit_vec(row.clone())
    }).collect_vec();

    let hashes_data = shards_commitments.iter()
        .flat_map(|e|  e.0.as_ref().iter())
        .flat_map(|e| {
            e.as_slice().iter()
        }).copied().collect_vec();

    let (root_shards_hash, _) = mmcs.commit_vec(hashes_data);

    let mut challenger = Poseidon2M31Challenger::new(POSEIDON2_M31_CONFIG.clone());
    let (commitment, _) = pcs_commit(vec![(commitment_domain, data.clone())]);

    challenger.observe(commitment);
    challenger.observe(root_shards_hash);

    let chi:Challenge = challenger.sample_ext_element();
    let evals = CircleEvaluations::from_natural_order(commitment_domain, data).evaluate_at_point(Point::from_projective_line(chi));

    (OptimisticCorrectableCommitment {
        pcs_commitment: commitment,
        shards_root_hash: root_shards_hash,
        opening_at_chi: evals
    }, shards)
}

// shards is matrix, where each row is a data from shard
//
pub fn recover_data<M:Matrix<Mersenne31>>(shards: M, subcoset_index:usize, log_blowup:usize) -> RowMajorMatrix<Mersenne31> {
    assert!(subcoset_index < (1<<log_blowup));

    let log_n = log2_strict_usize(shards.height());
    let source_domain = subdomain(subcoset_index, log_blowup, log_n);
    let target_domain = CircleDomain::<Mersenne31>::standard(log_n);
    let recovered_data = CircleEvaluations::from_natural_order(source_domain, shards).extrapolate(target_domain).to_natural_order();
    recovered_data.to_row_major_matrix().transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;


    #[test]
    fn test_points_over_subcoset() {
        let log_blowup = 3;
        let log_n=6;
        let subcoset_index = 2;

        let target_domain = CircleDomain::<Mersenne31>::standard(log_n+log_blowup);

        let subcoset_domain = subdomain(subcoset_index, log_blowup, log_n);

        let points = target_domain.points().collect_vec();
        let subcoset_points = subcoset_domain.points().collect_vec();

        let subcoset_indexes = subdomain_indexes(subcoset_index, log_blowup, log_n);

        let picked_points = subcoset_indexes.iter().map(|&i| points[i]).collect_vec();

        assert_eq!(picked_points, subcoset_points);
    }

    #[test]
    fn test_recover_data() {
        let mut rng = thread_rng();
        
        let log_blowup = 3;
        let log_n = 4;
        let log_h = 2;


        let source_data = RowMajorMatrix::<Mersenne31>::rand(&mut rng, 1<<log_h, 1<<log_n);

        let subcoset_index = rng.gen_range(0..(1<<log_blowup));

        let (_, shards) = commit(source_data.clone(), log_blowup);

        let subcoset_indexes = subdomain_indexes(subcoset_index, log_blowup, log_n);

        let subcoset_data = RowMajorMatrix::new(
            subcoset_indexes.iter().flat_map(|&i| shards[i].iter()).copied().collect_vec(),
            1<<log_h);

        let recovered_data = recover_data(subcoset_data, subcoset_index, log_blowup);

        assert_eq!(recovered_data, source_data);
    }

    #[test]
    fn test_evaluation_over_subcoset() {
        let mut rng = thread_rng();
        let log_blowup = 2;
        let log_w = 1;
        let log_h = 5;
        let subdomain_index = 2;

        let source_data = RowMajorMatrix::<Mersenne31>::rand(&mut rng, 1<<log_h, 1<<log_w);

        let source_data_domain = CircleDomain::<Mersenne31>::standard(log_h);
        let target_domain1 = CircleDomain::<Mersenne31>::standard(log_h+log_blowup);
        let target_domain2 = subdomain(subdomain_index, log_blowup, log_h);

        let blown_up_data = CircleEvaluations::from_natural_order(source_data_domain, source_data.clone()).extrapolate(target_domain1).to_natural_order();
        let subcoset_data = CircleEvaluations::from_natural_order(source_data_domain, source_data.clone()).extrapolate(target_domain2).to_natural_order();

        let subdomain_indexes = subdomain_indexes(subdomain_index, log_blowup, log_h);

        let data1 = subdomain_indexes.iter().flat_map(|&i| blown_up_data.row(i)).collect_vec();
        let data2 = subcoset_data.to_row_major_matrix().values;
        
        assert_eq!(data1, data2);
    }

    #[test]
    fn test_subdomain_indexes() {
        let log_blowup = 3;
        let log_n = 2;

        let blowup = 1 << log_blowup;
        let n_shards = 1 << (log_blowup + log_n);

        let mut all_indexes = Vec::new();

        for i in 0..blowup {
            let indexes = subdomain_indexes(i, log_blowup, log_n);
            all_indexes.extend(indexes);
        }


        all_indexes.sort_unstable();
        all_indexes.dedup();

        let expected: Vec<usize> = (0..n_shards).collect();

        assert_eq!(all_indexes, expected, "Merged subdomain indexes do not match the expected range 0..8*n");
    }
}