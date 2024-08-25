use alloc::vec;
use alloc::vec::Vec;

use itertools::{iterate, izip, Itertools};
use p3_commit::PolynomialSpace;
use p3_dft::{divide_by_height, Butterfly, DifButterfly, DitButterfly};
use p3_field::extension::ComplexExtendable;
use p3_field::{batch_multiplicative_inverse, ExtensionField, Field};
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use p3_maybe_rayon::prelude::*;
use p3_util::{log2_ceil_usize, log2_strict_usize, reverse_slice_index_bits};
use tracing::{debug_span, instrument};

use p3_circle::{CircleDomain, CircleEvaluations};
use p3_circle::Point;
use p3_circle::circle_basis;
use p3_circle::{cfft_permute_index, cfft_permute_slice, CfftPermutable, CfftView};

use p3_matrix::row_index_mapped::{RowIndexMap, RowIndexMappedView};




pub fn symmetric_cfft_interpolate<F: ComplexExtendable, M: Matrix<F>> (domain: CircleDomain<F>, evals: M) -> RowMajorMatrix<F> {
    assert_eq!(1 << (domain.log_n-1), evals.height());

    //TODO: rewrite inefficient matrix conversion and interpolation

    let symmetric_evals_rows = evals.rows().chain(evals.rows().collect_vec().into_iter().rev()).flatten().collect_vec();
    let symmetric_evals = RowMajorMatrix::new(symmetric_evals_rows, evals.width());

    //TODO: use cfft ordering for arguments instead of natural order
    let coeffs = CircleEvaluations::from_natural_order(domain, symmetric_evals).interpolate();

    let coeffs_even = coeffs.rows().step_by(2);
    let coeffs_odd = coeffs.rows().skip(1).step_by(2);

    // Odd coeffs should be zero
    assert!(coeffs_odd.flatten().all(|x| x.is_zero()));

    // Make matrix from even coeffs

    RowMajorMatrix::new(coeffs_even.flatten().collect_vec(), coeffs.width())
}

pub fn symmetric_cfft_evaluate<F: ComplexExtendable, M: Matrix<F>>(domain: CircleDomain<F>, coeffs: M) -> RowMajorMatrix<F> {
    assert_eq!(1 << (domain.log_n-1), coeffs.height());

    //TODO: rewrite inefficient matrix conversion and extrapolation

    let symmetric_coeffs = RowMajorMatrix::new(
        coeffs.rows().map(|row| row.collect_vec()).interleave_shortest(
            core::iter::repeat(vec![F::zero(); coeffs.width()])
        ).flatten().collect_vec(), coeffs.width());

    //TODO: use cfft ordering for result instead of natural order
    let evals = CircleEvaluations::evaluate(domain, symmetric_coeffs).to_natural_order();
    
    let symmetric_evals = RowMajorMatrix::new(evals.rows().take(evals.height()/2).flatten().collect_vec(), evals.width());

    assert!(evals.rows().collect_vec().into_iter().rev().take(evals.height()/2).flatten().zip(symmetric_evals.rows().flatten()).all(|(a, b)| a == b));

    symmetric_evals
}


// This function blows up each column of the matrix and resulting rows correspond to the data shards
// Domain customization is useful for sharding over cluster

pub fn shards_for_domain<F:ComplexExtendable, M:Matrix<F>>(source_domain:CircleDomain<F>, blowup_domain:CircleDomain<F>, source:M) -> RowMajorMatrix<F> {
    assert!(source_domain.size() <= blowup_domain.size());
    let coeffs = symmetric_cfft_interpolate(source_domain, source);
    let shards = symmetric_cfft_evaluate(blowup_domain, coeffs);
    shards
}

// The same as `do_shards_for_domain` but for standard domains, useful for sharding interpolation

pub fn shards<F:ComplexExtendable, M:Matrix<F>>(log_blowup:usize, source:M) -> RowMajorMatrix<F> {
    let source_domain = CircleDomain::<F>::standard(log2_strict_usize(source.height())+1);
    let blowup_domain = CircleDomain::<F>::standard(log_blowup + log2_ceil_usize(source.height()) + 1);
    shards_for_domain(source_domain, blowup_domain, source)
}

// Assumes that source is evaluated over standard domain
// Computes the polynomial related to https://ethresear.ch/t/efficient-data-distribution-with-reed-solomon-codes-for-sharded-storage
// and returns its coefficients representation
pub fn data_polynomial_coeffs_repr<F:ComplexExtendable, M:Matrix<F>>(source:M) -> RowMajorMatrix<F> {
    
    // transpose repr, so rows and cols below are vice versa
    let a = source;

    // First perform fft over columns of source matrix with m rows and n columns, where both m and n are powers of 2
    // f(X,Y) = ∑ aᵢⱼ Lᵢ(X) λⱼ(Y)
    //          ⁱʲ
    // where X and Y are points over the circle 
    // i is the row index and j is the column index
    // Lᵢ(X) is the lagrange polynomial for the i-th row
    // λⱼ(Y) is the lagrange polynomial for the j-th column
    //
    // After the transformation we got
    // f(X,Y) = ∑ bᵢⱼ Lᵢ(X) μⱼ(Y)
    //          ⁱʲ
    // where μⱼ(Y) is element of symmetric monomial basis
    // μₖ(P) = cpow(P.x, k) = ∏ π⁽ʲ⁾(P.x)
    //                      bitⱼ(k) = 1 
    // π(x) = 2 x² −1

    let b = symmetric_cfft_interpolate(CircleDomain::<F>::standard(log2_ceil_usize(a.height())+1), a); 

    // next do interpolation over rows
    // f(X,Y) = ∑ bᵢⱼ Lᵢ(X) μⱼ(Y) = ∑ cᵢⱼ Mᵢ(X) μⱼ(Y) = ∑ fⱼ(X) μⱼ(Y)
    //          ⁱʲ                  ⁱʲ                  ʲ
    // where Mᵢ(X) = yⁱ ᵐᵒᵈ ² μᵢ÷₂(X)  is general monomial basis, including both y-symmetric and y-antisymmetric parts
    // Taking into account, that cpow(cpow(x, α), β)=cpow(x, αβ), when α is a power of 2, 
    // and cpow(x, α) cpow(x, β) = cpow(x, α+β), when α xor β = 0, we can do the substitution:
    // Y=Xᵐ, then
    // f(X, Xᵐ) = ∑ cᵢⱼ Mᵢ(X) μⱼ(Xᵐ) = ∑ cᵢⱼ Mᵢ+ₘⱼ(X)
    //            ⁱʲ                   ⁱʲ  
    // that means that to build coefficient representation of f(X, Xᵐ) we need to do cfft over cols and concatenate the resulting cols one by one

    
    // make transpose to make matrix representation the same as in the paper
    // we need it because only cfft over cols is implemented in Plonky3
    let b = b.transpose();

    
    let coeffs = CircleEvaluations::from_natural_order(
        CircleDomain::<F>::standard(log2_ceil_usize(b.height())), b
    ).interpolate();

    // concatenate all coeffs into one single-column row-major matrix

    let coeffs = RowMajorMatrix::new(coeffs.transpose().values, 1);

    // // TODO: this part should be moved to FRI commitment computation
    // CircleEvaluations::evaluate(
    //     CircleDomain::<F>::standard(log2_ceil_usize(coeffs.height())), 
    //     coeffs).to_natural_order().to_row_major_matrix()

    coeffs
    
}


// Solves f(X)-fₛ(X) = v₀(XᵐY₀⁻¹)q(X) and returns coordinate lagrange representation of q(X)
// Y₀ is corresponding to the shard fₛ
// source and shard should be given in monomial representation
// output is in lagrange representation
fn quotient_polynomial_evals<F:ComplexExtendable, M:Matrix<F>>(source:M, shard:M, y_0:Point<F>) -> RowMajorMatrix<F> {

    //implemented only for 1-column source matrix
    assert!(source.width()==1);
    assert!(shard.width()==1);

    assert_eq!(source.height()%shard.height(), 0);
    let m = source.height()/shard.height();


    // compute f(X)-fₛ(X)

    // this part could be optimized by memory

    let source_height = source.height();

    let source_values = source.to_row_major_matrix().values;
    let shard_values = shard.to_row_major_matrix().values;

    let f_minus_f_s_coeffs = RowMajorMatrix::new(
        source_values.iter().zip(shard_values.iter()).map(|(&a, &b)| a-b).chain(source_values.iter().skip(shard_values.len()).cloned()).collect_vec(), 1
    );

    let domain = CircleDomain::<F>::standard(log2_ceil_usize(source_height));

    let f_minus_f_s_evals = CircleEvaluations::evaluate(domain, f_minus_f_s_coeffs).to_natural_order().to_row_major_matrix();


    // This part could be optimized by using properties of Xᵐ
    let points = domain.points();
    let v_0_args = points.map(|x| v_0(x, m, y_0)).collect_vec();

    let v_0_noms = v_0_args.iter().map(|p| p.y).collect_vec();
    let v_0_denoms = v_0_args.iter().map(|p| p.x+F::one()).collect_vec();

    let v_0_noms_inv = batch_multiplicative_inverse(&v_0_noms);

    RowMajorMatrix::new(
        f_minus_f_s_evals.values.iter().zip(v_0_noms_inv.iter().zip(v_0_denoms.iter())).map(
            |(&a, (&b, &c))| a*b*c
        ).collect_vec(), 1
    )
    
}

//v₀(XᵐY₀⁻¹) for testing purposes only
fn v_0<F:Field>(x:Point<F>, m:usize, y_0:Point<F>) -> Point<F> {
    x*m - y_0
}

mod tests {
    use itertools::iproduct;
    use p3_field::extension::BinomialExtensionField;
    use p3_mersenne_31::Mersenne31;
    use rand::{random, thread_rng};

    use super::*;

    type F = Mersenne31;
    type EF = BinomialExtensionField<F, 3>;

    #[test]
    fn test_symmetric_cfft_icfft() {
        for (log_n, width) in iproduct!(2..5, [1, 4, 11]) {
            let domain = CircleDomain::<F>::standard(log_n+1);
            let trace = RowMajorMatrix::rand(&mut thread_rng(), 1 << log_n, width);
            let coeffs = symmetric_cfft_interpolate(domain, trace.clone());
            let evals = symmetric_cfft_evaluate(domain, coeffs);
            assert_eq!(evals, trace, "symmetric_cfft_evaluate(symmetric_cfft_interpolate(evals)) is identity");
        }
    }

    /* 
    #[test]
    fn test_cfft_icfft() {
        for (log_n, width) in iproduct!(2..5, [1, 4, 11]) {
            let shift = Point::generator(F::CIRCLE_TWO_ADICITY) * random();
            let domain = CircleDomain::<F>::new(log_n, shift);
            let trace = RowMajorMatrix::<F>::rand(&mut thread_rng(), 1 << log_n, width);
            let coeffs = CircleEvaluations::from_natural_order(domain, trace.clone()).interpolate();
            assert_eq!(
                CircleEvaluations::evaluate(domain, coeffs.clone())
                    .to_natural_order()
                    .to_row_major_matrix(),
                trace,
                "icfft(cfft(evals)) is identity",
            );
            for (i, pt) in domain.points().enumerate() {
                assert_eq!(
                    &*trace.row_slice(i),
                    coeffs.columnwise_dot_product(&circle_basis(pt, log_n)),
                    "coeffs can be evaluated with circle_basis",
                );
            }
        }
    }

    #[test]
    fn test_extrapolation() {
        for (log_n, log_blowup) in iproduct!(2..5, [1, 2, 3]) {
            let evals = CircleEvaluations::<F>::from_natural_order(
                CircleDomain::standard(log_n),
                RowMajorMatrix::rand(&mut thread_rng(), 1 << log_n, 11),
            );
            let lde = evals
                .clone()
                .extrapolate(CircleDomain::standard(log_n + log_blowup));

            let coeffs = evals.interpolate();
            let lde_coeffs = lde.interpolate();

            for r in 0..coeffs.height() {
                assert_eq!(&*coeffs.row_slice(r), &*lde_coeffs.row_slice(r));
            }
            for r in coeffs.height()..lde_coeffs.height() {
                assert!(lde_coeffs.row(r).all(|x| x.is_zero()));
            }
        }
    }

    #[test]
    fn eval_at_point_matches_cfft() {
        for (log_n, width) in iproduct!(2..5, [1, 4, 11]) {
            let evals = CircleEvaluations::<F>::from_natural_order(
                CircleDomain::standard(log_n),
                RowMajorMatrix::rand(&mut thread_rng(), 1 << log_n, width),
            );

            let pt = Point::<EF>::from_projective_line(random());

            assert_eq!(
                evals.clone().evaluate_at_point(pt),
                evals
                    .interpolate()
                    .columnwise_dot_product(&circle_basis(pt, log_n))
            );
        }
    }

    #[test]
    fn eval_at_point_matches_lde() {
        for (log_n, width, log_blowup) in iproduct!(2..8, [1, 4, 11], [1, 2]) {
            let evals = CircleEvaluations::<F>::from_natural_order(
                CircleDomain::standard(log_n),
                RowMajorMatrix::rand(&mut thread_rng(), 1 << log_n, width),
            );
            let lde = evals
                .clone()
                .extrapolate(CircleDomain::standard(log_n + log_blowup));
            let zeta = Point::<EF>::from_projective_line(random());
            assert_eq!(evals.evaluate_at_point(zeta), lde.evaluate_at_point(zeta));
            assert_eq!(
                evals.evaluate_at_point(zeta),
                evals
                    .interpolate()
                    .columnwise_dot_product(&circle_basis(zeta, log_n))
            );
            assert_eq!(
                lde.evaluate_at_point(zeta),
                lde.interpolate()
                    .columnwise_dot_product(&circle_basis(zeta, log_n + log_blowup))
            );
        }
    }
    */
}
