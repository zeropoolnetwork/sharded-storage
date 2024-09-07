use alloc::vec;
use alloc::vec::Vec;
use core::iter::repeat;
use itertools::Itertools;
use p3_commit::PolynomialSpace;
use p3_field::extension::ComplexExtendable;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use p3_maybe_rayon::prelude::*;
use p3_util::log2_strict_usize;


use p3_circle::{CircleDomain, CircleEvaluations};
use p3_circle::Point;


//use libc_print::std_name::{println, eprintln, dbg};

use core::ops::Sub;


// Return symmetric monomial basis at given point
// log_n is corresponding to the size of full basis size, 
// including both y-symmetric and y-antisymmetric parts

pub fn symmetric_circle_basis<F: Field>(p: Point<F>, log_n: usize) -> Vec<F> {
    let mut b = vec![F::one()];
    let mut x = p.x;
    for _ in 0..(log_n-1) {
        for i in 0..b.len() {
            b.push(b[i] * x);
        }
        x = x.square().double() - F::one();
    }
    assert_eq!(b.len(), 1 << (log_n-1));
    b
}


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
    let coeffs_height = coeffs.height();
    let log_coeffs_height = log2_strict_usize(coeffs_height);

    assert!(domain.log_n > log_coeffs_height);

    let symmetric_coeffs_len = coeffs.width() * (1<<domain.log_n);

    //TODO: rewrite inefficient matrix conversion and extrapolation
    //TODO: optimize cfft over partially zeroized data (see examples in Plonky3 cfft implementation)

    let symmetric_coeffs = RowMajorMatrix::new(
        coeffs.rows().map(|row| row.collect_vec()).interleave_shortest(
            repeat(vec![F::zero(); coeffs.width()])
        ).flatten().chain(repeat(F::zero())).take(symmetric_coeffs_len).collect_vec(), coeffs.width());

    //TODO: use cfft ordering for result instead of natural order
    let evals = CircleEvaluations::evaluate(domain, symmetric_coeffs).to_natural_order();
    
    let symmetric_evals = RowMajorMatrix::new(evals.rows().take(evals.height()/2).flatten().collect_vec(), evals.width());

    assert!(evals.rows().collect_vec().into_iter().rev().take(evals.height()/2).flatten().zip(symmetric_evals.rows().flatten()).all(|(a, b)| a == b));

    symmetric_evals
}


// This function blows up each column of the matrix and resulting rows correspond to the data shards in lagrange representation
// Domain customization is useful for sharding over cluster

pub fn shards_for_domain<F:ComplexExtendable, M:Matrix<F>>(source_domain:CircleDomain<F>, blowup_domain:CircleDomain<F>, source:M) -> RowMajorMatrix<F> {
    assert!(source_domain.size() <= blowup_domain.size());
    let coeffs = symmetric_cfft_interpolate(source_domain, source);
    symmetric_cfft_evaluate(blowup_domain, coeffs)
}

// The same as `do_shards_for_domain` but for standard domains, useful for sharding interpolation

pub fn shards<F:ComplexExtendable, M:Matrix<F>>(log_blowup:usize, source:M) -> RowMajorMatrix<F> {
    let source_domain = CircleDomain::<F>::standard(log2_strict_usize(source.height())+1);
    let blowup_domain = CircleDomain::<F>::standard(log_blowup + log2_strict_usize(source.height()) + 1);
    shards_for_domain(source_domain, blowup_domain, source)
}


// Assumes that source is evaluated over standard domain
// Computes the polynomial related to https://ethresear.ch/t/efficient-data-distribution-with-reed-solomon-codes-for-sharded-storage
// and returns its coefficients representation
pub fn data_polynomial_coeffs<F:ComplexExtendable, M:Matrix<F>>(source:M) -> RowMajorMatrix<F> {
    // f(X,Y) = ∑ cᵢⱼ Mᵢ(X) μⱼ(Y) = ∑ fⱼ(X) μⱼ(Y)
    //          ⁱʲ                  ʲ     
    // where μⱼ(Y) is element of symmetric monomial basis
    // μₖ(P) = cpow(P.x, k) = ∏ π⁽ʲ⁾(P.x)
    //                      bitⱼ(k) = 1 
    // π(x) = 2 x² −1
    // and Mᵢ(X) = yⁱ ᵐᵒᵈ ² μᵢ÷₂(X)  is general monomial basis, including both y-symmetric and y-antisymmetric parts

    // Taking into account, that cpow(cpow(x, α), β)=cpow(x, αβ), when α is a power of 2, 
    // and cpow(x, α) cpow(x, β) = cpow(x, α+β), when α xor β = 0, we can do the substitution:
    // Y=Xᵐ, then
    // f(X, Xᵐ) = ∑ aᵢⱼ Mᵢ(X) μⱼ(Xᵐ) = ∑ aᵢⱼ Mᵢ+ₘⱼ(X)
    //            ⁱʲ                   ⁱʲ  
    // that means that to build coefficient representation of f(X, Xᵐ) we need to do cfft over cols and concatenate the resulting cols one by one


    RowMajorMatrix::new(
        source.to_row_major_matrix().transpose().values, 1
    )
}


// Solves f(X)-fₛ(X) = v₀(XᵐY₀⁻¹)q(X) and returns coordinate lagrange representation of q(X)
// Y₀ is corresponding to the shard fₛ
// source and shard should be given in monomial representation
// output is in lagrange representation
pub fn quotient_polynomial_evals<F:ComplexExtendable, M:Matrix<F>>(data:M, shard:M, y_0:Point<F>) -> RowMajorMatrix<F> {

    let data_height = data.height();
    let shard_height = shard.height();
    let log_data_height = log2_strict_usize(data_height);

    assert!(data.width()==1);
    assert!(shard.width()==1);

    let data_values = data.to_row_major_matrix().values;
    let shard_values = shard.to_row_major_matrix().values;

    let f_minus_f_s_coeffs = RowMajorMatrix::new(
        data_values.iter().zip(shard_values.iter()).map(|(&a, &b)| a-b).chain(data_values.iter().skip(shard_height).cloned()).collect_vec(), 1
    );

    let quotient_domain = CircleDomain::<F>::standard(log_data_height+1);

    let f_minus_f_s_evals = CircleEvaluations::evaluate(quotient_domain, f_minus_f_s_coeffs).to_natural_order().to_row_major_matrix().values;


    // TODO: optimize it by using montgomery batch inverse and compute separately nominator and denominator of v₀
    // TODO: Optimize b*shard_height/2 here using group properties
    let quotient_evals = f_minus_f_s_evals.iter().zip(quotient_domain.points()).map(|(&a, b)| a/v_0(b, shard_height/2, y_0 ).to_projective_line().unwrap()).collect_vec();

    RowMajorMatrix::new(quotient_evals, 1)

}


//v₀(XᵐY₀⁻¹) for testing purposes only
fn v_0<F:Field, G:Field>(x:Point<F>, m:usize, y_0:Point<G>) -> Point<F> 
    where Point<F>:Sub<Point<G>, Output = Point<F>>
{
    x*m-y_0
}

#[cfg(test)]
mod tests {


    use itertools::iproduct;
    use p3_field::extension::BinomialExtensionField;
    use p3_mersenne_31::Mersenne31;
    use p3_circle::circle_basis;
    use rand::{thread_rng, Rng};

    use super::*;

    type F = Mersenne31;
    type EF = BinomialExtensionField<F, 3>;

    #[test]
    fn test_opening() {
        let mut rng = thread_rng();
        let log2_h = 5;
        let log_lde_blowup = 1;

        let log2_lde = log2_h+log_lde_blowup;
        let h = 1<<log2_h;


        let coeffs = RowMajorMatrix::<F>::rand(&mut rng, h, 1);

        let domain = CircleDomain::<F>::standard(log2_lde);

        let evals = CircleEvaluations::evaluate(domain, coeffs.clone()).to_natural_order().to_row_major_matrix();

        let x = Point::<F>::from_projective_line(rng.gen());

        let value = coeffs.columnwise_dot_product(&circle_basis(x, log2_h))[0];
        let value2 = CircleEvaluations::from_natural_order(domain, evals.clone()).evaluate_at_point(x)[0];

        assert_eq!(value, value2);

        let opening = value;

        let evals_minus_opening = evals.values.clone().into_iter().map(|a| a-opening).collect_vec();

        assert_eq!(evals_minus_opening.len(), domain.points().collect_vec().len());

        
        
        let quotient_values = evals_minus_opening.clone().into_iter().zip(domain.points()).map(
            |(a, b)| {
                let v_0_eval = v_0(b, 1, x);
                a / v_0_eval.to_projective_line().unwrap()
            }).collect_vec();

        let quotient = CircleEvaluations::from_natural_order(domain, RowMajorMatrix::new(quotient_values, 1));


        let x2 = Point::<EF>::from_projective_line(rng.gen());

        let quotient_at_x2 = quotient.evaluate_at_point(x2)[0];
        let evals_at_x2 = CircleEvaluations::from_natural_order(domain, evals.clone()).evaluate_at_point(x2)[0];
        let v_0_eval_at_x2 = v_0(x2, 1, x).to_projective_line().unwrap();

        assert_eq!(quotient_at_x2*v_0_eval_at_x2, evals_at_x2-opening);

    }

    
    #[test]
    fn test_sharding() {

        let mut rng = thread_rng();

        let height = 32; // size of shard
        let width = 8; // number of shards to restore source data

        let num_shards = 32;
        let log_num_shards = log2_strict_usize(num_shards);
        let shard_index = 5; // select n-th shard to test numerated from 0

        let log_height = log2_strict_usize(height);
        let log_width = log2_strict_usize(width);
        let log_data_height = log_height + log_width;

        let sharding_domain = CircleDomain::<F>::standard(log_num_shards+1);

        let quotient_domain = CircleDomain::<F>::standard(log_data_height+1);
        

        let source = RowMajorMatrix::<F>::rand(&mut rng, height, width);


        let source_transposed = source.clone().transpose();

        
        
    

        let y_0 = sharding_domain.points().nth(shard_index).unwrap();

        // +1 because of y-symmetry
        let shard = RowMajorMatrix::new(
            source_transposed.columnwise_dot_product(&symmetric_circle_basis(y_0, log_num_shards+1)), 1
        );

        assert!(shard.height()==height);

        let data_coeffs = data_polynomial_coeffs(source);

        let quotient_evals = quotient_polynomial_evals(data_coeffs.clone(), shard.clone(), y_0);

        let x = Point::<EF>::from_projective_line(rng.gen());

        

        let data_eval_at_x = data_coeffs.columnwise_dot_product(&circle_basis(x, log_data_height))[0];

        let quotient_eval_at_x = CircleEvaluations::from_natural_order(quotient_domain, quotient_evals).evaluate_at_point(x)[0];

        let shard_eval_at_x = shard.columnwise_dot_product(&circle_basis(x, log_height))[0];

        let v_0_eval_at_x = v_0(x, height/2, y_0).to_projective_line().unwrap();



        assert_eq!(data_eval_at_x-shard_eval_at_x, quotient_eval_at_x*v_0_eval_at_x);
    }


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

}
