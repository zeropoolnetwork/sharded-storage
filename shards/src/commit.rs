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

/// Represents an optimistic correctable commitment with PCS commitment, root hash of shards, and opening evaluations at a challenge point.
pub struct OptimisticCorrectableCommitment {
    /// PCS commitment hash
    pub pcs_commitment_hash: Hash,
    /// Root hash of all shards
    pub shards_root_hash: Hash,
    /// Opening evaluations at challenge chi
    pub opening_evaluations: Vec<Challenge>,
}

/// Returns a subdomain for efficient data recovery.
///
/// # Arguments
///
/// * `index` - The index of the subdomain.
/// * `log_blowup_factor` - The logarithm of the blowup factor.
/// * `log_dimension` - The logarithm of the dimension.
///
/// # Panics
///
/// Panics if `index` is not less than `2^log_blowup_factor`.
///
/// # Returns
///
/// A new `CircleDomain` representing the subdomain.
#[must_use]
pub fn compute_subdomain(index: usize, log_blowup_factor: usize, log_dimension: usize) -> CircleDomain<Mersenne31> {
    assert!(index < (1 << log_blowup_factor), "Subdomain index out of bounds");

    let shift_point = Point::generator(log_dimension + log_blowup_factor + 1);
    let generator_point = Point::generator(log_dimension + log_blowup_factor);

    CircleDomain::new(log_dimension, shift_point + generator_point * index)
}

/// Returns the indexes for a given subdomain to facilitate data recovery.
///
/// # Arguments
///
/// * `index` - The index of the subdomain.
/// * `log_blowup_factor` - The logarithm of the blowup factor.
/// * `log_dimension` - The logarithm of the dimension.
///
/// # Returns
///
/// A vector of `usize` representing the indexes in the subdomain.
#[must_use]
pub fn compute_subdomain_indexes(index: usize, log_blowup_factor: usize, log_dimension: usize) -> Vec<usize> {
    let blowup = 1 << log_blowup_factor;
    let half_shards = 1 << (log_dimension - 1);

    let subcoset0_indexes = iterate(index, |&i| i + 2 * blowup).take(half_shards);
    let subcoset1_indexes = iterate(2 * blowup - index - 1, |&i| i + 2 * blowup).take(half_shards);
    subcoset0_indexes.interleave(subcoset1_indexes).collect_vec()
}

/// Computes all shards and commitment as described in [ETH research](https://ethresear.ch/t/using-fri-for-da-with-optimistic-correctable-commitments-in-rollups/20467).
///
/// # Arguments
///
/// * `data_matrix` - The input data matrix.
/// * `log_blowup_factor` - The logarithm of the blowup factor.
///
/// # Returns
///
/// A tuple containing:
/// - `OptimisticCorrectableCommitment`: The computed commitment.
/// - `Vec<Vec<Mersenne31>>`: The generated shards.
pub fn compute_commitment<M: Matrix<Mersenne31>>(data_matrix: M, log_blowup_factor: usize) -> (OptimisticCorrectableCommitment, Vec<Vec<Mersenne31>>) {
    let mmcs = POSEIDON2_M31_MMCS.clone();

    let data_width = data_matrix.width();
    let data_height = data_matrix.height();
    let log_data_width = log2_strict_usize(data_width);
    let log_num_shards = log_data_width + log_blowup_factor;
    let log_data_height = log2_strict_usize(data_height);

    let data_domain = CircleDomain::<Mersenne31>::standard(log_data_width);
    let shards_domain = CircleDomain::<Mersenne31>::standard(log_num_shards);
    let commitment_domain = CircleDomain::<Mersenne31>::standard(log_data_height);
    let row_major_data = data_matrix.to_row_major_matrix();
    let transposed_data = row_major_data.transpose();

    // OPTIMIZATION TIPS:
    // 1. Replace source representation from evaluations to coefficients (for sharding only, for commitment use evaluations)
    // 2. Store result in FFT order

    let expanded_data = CircleEvaluations::from_natural_order(data_domain, transposed_data)
        .extrapolate(shards_domain)
        .to_natural_order();

    let shards = expanded_data.par_rows().map(|row| row.collect_vec()).collect::<Vec<_>>();

    let shard_commitments = shards.iter().map(|row| mmcs.commit_vec(row.clone())).collect_vec();

    let concatenated_hashes = shard_commitments.iter()
        .flat_map(|commitment| commitment.0.as_ref().iter())
        .flat_map(|hash_chunk| hash_chunk.as_slice().iter())
        .copied()
        .collect_vec();

    let (root_shards_hash, _) = mmcs.commit_vec(concatenated_hashes);

    let mut challenger = Poseidon2M31Challenger::new(POSEIDON2_M31_CONFIG.clone());
    let (pcs_commitment, _) = pcs_commit(vec![(commitment_domain, row_major_data.clone())]);

    challenger.observe(pcs_commitment);
    challenger.observe(root_shards_hash);

    let challenge_chi: Challenge = challenger.sample_ext_element();
    let evaluations = CircleEvaluations::from_natural_order(commitment_domain, row_major_data)
        .evaluate_at_point(Point::from_projective_line(challenge_chi));

    (
        OptimisticCorrectableCommitment {
            pcs_commitment_hash: pcs_commitment,
            shards_root_hash: root_shards_hash,
            opening_evaluations: evaluations,
        },
        shards,
    )
}

/// Recovers the original data from shards using the specified subcoset index.
///
/// # Arguments
///
/// * `shards_matrix` - The matrix of shards where each row corresponds to a shard.
/// * `subcoset_index` - The index of the subcoset to use for recovery.
/// * `log_blowup_factor` - The logarithm of the blowup factor.
///
/// # Panics
///
/// Panics if `subcoset_index` is not less than `2^log_blowup_factor`.
///
/// # Returns
///
/// The recovered data as a row-major matrix.
pub fn recover_original_data<M: Matrix<Mersenne31>>(shards_matrix: M, subcoset_index: usize, log_blowup_factor: usize) -> RowMajorMatrix<Mersenne31> {
    assert!(subcoset_index < (1 << log_blowup_factor), "Subcoset index out of bounds");

    let log_dimension = log2_strict_usize(shards_matrix.height());
    let source_domain = compute_subdomain(subcoset_index, log_blowup_factor, log_dimension);
    let target_domain = CircleDomain::<Mersenne31>::standard(log_dimension);
    let recovered_evaluations = CircleEvaluations::from_natural_order(source_domain, shards_matrix)
        .extrapolate(target_domain)
        .to_natural_order();
    recovered_evaluations.to_row_major_matrix().transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    /// Tests that points over a subcoset are correctly selected from the target domain.
    #[test]
    fn test_subcoset_points_selection() {
        let log_blowup_factor = 3;
        let log_dimension = 6;
        let subcoset_index = 2;

        let target_domain = CircleDomain::<Mersenne31>::standard(log_dimension + log_blowup_factor);
        let subcoset_domain = compute_subdomain(subcoset_index, log_blowup_factor, log_dimension);

        let all_points = target_domain.points().collect_vec();
        let subcoset_points = subcoset_domain.points().collect_vec();

        let subcoset_indexes = compute_subdomain_indexes(subcoset_index, log_blowup_factor, log_dimension);

        let selected_points = subcoset_indexes.iter().map(|&i| all_points[i]).collect_vec();

        assert_eq!(selected_points, subcoset_points, "Selected subcoset points do not match expected subcoset points");
    }

    /// Tests the data recovery process from shards.
    #[test]
    fn test_data_recovery() {
        let mut rng = thread_rng();
        
        let log_blowup_factor = 3;
        let log_dimension = 4;
        let log_height = 2;

        let original_data = RowMajorMatrix::<Mersenne31>::rand(&mut rng, 1 << log_height, 1 << log_dimension);
        let subcoset_index = rng.gen_range(0..(1 << log_blowup_factor));

        let (_, shards) = compute_commitment(original_data.clone(), log_blowup_factor);

        let subcoset_indexes = compute_subdomain_indexes(subcoset_index, log_blowup_factor, log_dimension);

        let subcoset_data = RowMajorMatrix::new(
            subcoset_indexes.iter().flat_map(|&i| shards[i].iter()).copied().collect_vec(),
            1 << log_height,
        );

        let recovered_data = recover_original_data(subcoset_data, subcoset_index, log_blowup_factor);

        assert_eq!(recovered_data, original_data, "Recovered data does not match the original data");
    }

    /// Tests that evaluations over a subcoset are consistent with the expanded data.
    #[test]
    fn test_evaluation_over_subcoset() {
        let mut rng = thread_rng();
        let log_blowup_factor = 2;
        let log_width = 1;
        let log_height = 5;
        let subdomain_index = 2;

        let source_data = RowMajorMatrix::<Mersenne31>::rand(&mut rng, 1 << log_height, 1 << log_width);

        let source_domain = CircleDomain::<Mersenne31>::standard(log_height);
        let expanded_domain = CircleDomain::<Mersenne31>::standard(log_height + log_blowup_factor);
        let subcoset_domain = compute_subdomain(subdomain_index, log_blowup_factor, log_height);

        let expanded_data = CircleEvaluations::from_natural_order(source_domain, source_data.clone())
            .extrapolate(expanded_domain)
            .to_natural_order();
        let subcoset_data = CircleEvaluations::from_natural_order(source_domain, source_data.clone())
            .extrapolate(subcoset_domain)
            .to_natural_order();

        let subdomain_indexes = compute_subdomain_indexes(subdomain_index, log_blowup_factor, log_height);

        let data_from_expanded = subdomain_indexes.iter().flat_map(|&i| expanded_data.row(i)).collect_vec();
        let data_from_subcoset = subcoset_data.to_row_major_matrix().values;
        
        assert_eq!(data_from_expanded, data_from_subcoset, "Data extracted from expanded domain does not match data from subcoset domain");
    }

    /// Tests that all subdomain indexes cover the expected range without duplicates.
    #[test]
    fn test_subdomain_indexes_coverage() {
        let log_blowup_factor = 3;
        let log_dimension = 2;

        let blowup = 1 << log_blowup_factor;
        let total_shards = 1 << (log_blowup_factor + log_dimension);

        let mut all_indexes = Vec::new();

        for i in 0..blowup {
            let indexes = compute_subdomain_indexes(i, log_blowup_factor, log_dimension);
            all_indexes.extend(indexes);
        }

        all_indexes.sort_unstable();
        all_indexes.dedup();

        let expected_indexes: Vec<usize> = (0..total_shards).collect();

        assert_eq!(all_indexes, expected_indexes, "Merged subdomain indexes do not cover the expected range 0..8*n");
    }
}
