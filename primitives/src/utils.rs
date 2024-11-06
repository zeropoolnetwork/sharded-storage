use p3_symmetric::CryptographicPermutation;
use p3_circle::CircleDomain;
use p3_commit::Pcs;
use alloc::vec::Vec;
use p3_field::{Field, batch_multiplicative_inverse};
use p3_matrix::{Matrix, dense::RowMajorMatrix};
use p3_maybe_rayon::prelude::*;
use core::ops::Deref;
use alloc::vec;
use itertools::Itertools;
use core::iter::Iterator;

use crate::config::{Challenge, Poseidon2Challenger, Poseidon2Pcs, pcs_config, Val};

// Function to invert a square RowMajorMatrix
pub fn invert_matrix<T: Field>(matrix: &RowMajorMatrix<T>) -> RowMajorMatrix<T> {
    let n = matrix.width();
    assert_eq!(n, matrix.height(), "Matrix must be square");

    // Create mutable copies of the matrix and the identity matrix
    let mut a = matrix.clone();
    let mut inv = identity_matrix(n);

    let width = a.width();

    for i in 0..n {
        // Partial pivoting
        let mut pivot_row = i;
        for j in i..n {
            if !a.get(j, i).is_zero() {
                pivot_row = j;
                break;
            }
        }
        if a.get(pivot_row, i).is_zero() {
            panic!("Matrix is singular and cannot be inverted");
        }

        // Swap rows if needed
        if i != pivot_row {
            swap_rows(&mut a, i, pivot_row);
            swap_rows(&mut inv, i, pivot_row);
        }

        // Get the pivot element
        let pivot = a.get(i, i);

        let pivot_inv = pivot.inverse();

        // Scale the pivot row
        scale_row(&mut a, i, pivot_inv);
        scale_row(&mut inv, i, pivot_inv);

        // Get immutable copies of the pivot row
        let start_i = i * width;
        let end_i = start_i + width;
        let row_i_a = a.values[start_i..end_i].to_vec();
        let row_i_inv = inv.values[start_i..end_i].to_vec();

        // Eliminate the current column entries in other rows
        for j in 0..n {
            if j != i {
                let factor = a.get(j, i);
                if !factor.is_zero() {
                    // Get mutable slices for row j
                    let start_j = j * width;
                    let end_j = start_j + width;

                    let row_j_a = &mut a.values[start_j..end_j];
                    let row_j_inv = &mut inv.values[start_j..end_j];

                    // Update row_j_a in parallel
                    row_j_a
                        .par_iter_mut()
                        .zip(row_i_a.par_iter())
                        .for_each(|(t, &s)| {
                            *t -= factor * s;
                        });

                    // Update row_j_inv in parallel
                    row_j_inv
                        .par_iter_mut()
                        .zip(row_i_inv.par_iter())
                        .for_each(|(t, &s)| {
                            *t -= factor * s;
                        });
                }
            }
        }
    }

    inv
}


// Helper function to create an identity matrix
fn identity_matrix<T: Field>(n: usize) -> RowMajorMatrix<T> {
    let mut values = vec![T::zero(); n * n];
    for i in 0..n {
        values[i * n + i] = T::one();
    }
    RowMajorMatrix::new(values, n)
}

// Helper function to swap two rows in a matrix
fn swap_rows<T:Field>(matrix: &mut RowMajorMatrix<T>, row1: usize, row2: usize) {
    if row1 == row2 {
        return;
    }
    let width = matrix.width();
    let start1 = row1 * width;
    let start2 = row2 * width;

    for i in 0..width {
        matrix.values.swap(start1 + i, start2 + i);
    }
}

// Helper function to scale a row by a scalar
fn scale_row<T:Field>(matrix: &mut RowMajorMatrix<T>, row: usize, scalar: T) {
    let width = matrix.width();
    let start = row * width;
    let end = start + width;
    let row_slice = &mut matrix.values[start..end];

    row_slice
        .par_iter_mut()
        .for_each(|val| *val *= scalar);
}


// Function to multiply two matrices using columnwise_dot_product
pub fn multiply_matrices<T:Field>(
    a: &RowMajorMatrix<T>,
    b: &RowMajorMatrix<T>,
) -> RowMajorMatrix<T> {
    assert_eq!(
        a.width(),
        b.height(),
        "Incompatible dimensions for multiplication"
    );

    let m = a.height();
    let p = b.width();

    let mut result = RowMajorMatrix::default(p, m); // Create m x p matrix

    result
        .values
        .par_chunks_mut(p)
        .enumerate()
        .for_each(|(i, row)| {
            row.copy_from_slice(&b.columnwise_dot_product(a.row_slice(i).deref()));
        });

    result
}



type PcsCommitment = <Poseidon2Pcs as Pcs::<Challenge,Poseidon2Challenger>>::Commitment;
type PcsProverData = <Poseidon2Pcs as Pcs::<Challenge,Poseidon2Challenger>>::ProverData;


pub fn pcs_commit(data: Vec<(CircleDomain<Val>,RowMajorMatrix<Val>)>) -> (PcsCommitment, PcsProverData)  {
    Pcs::<Challenge,Poseidon2Challenger>::commit(&pcs_config(), data)
}

#[derive(Clone, Debug)]
pub struct StreamCipher<T, P, const WIDTH: usize, const OUT: usize>
{
    permutation: P,
    phantom: core::marker::PhantomData<T>
}

impl <T, P, const WIDTH: usize, const OUT: usize> StreamCipher<T, P,WIDTH,OUT>
where T: Default+Copy
{
    pub fn new(permutation: P) -> Self {
        Self {
            permutation,
            phantom: core::marker::PhantomData
        }
    }

    pub fn cipher(&self, state: &[T]) -> StreamCipherIterator<T,P,WIDTH,OUT> {
        StreamCipherIterator::new(self, state)
    }
}

#[derive(Clone, Debug)]
pub struct StreamCipherIterator<'a, T, P, const WIDTH: usize, const OUT: usize> {
    cipher: &'a StreamCipher<T,P,WIDTH,OUT>,
    state: [T;WIDTH],
    index:usize
}



impl <'a, T, P, const WIDTH: usize, const OUT: usize> StreamCipherIterator<'a, T,P,WIDTH,OUT>
where T: Default+Copy
{
    pub fn new(cipher: &'a StreamCipher<T,P,WIDTH,OUT>, state: &[T]) -> Self {
        assert!(state.len() <= WIDTH, "State length must be less than or equal to WIDTH");
        let mut state_array = [T::default(); WIDTH];
        state_array[..state.len()].copy_from_slice(state);
        Self {
            cipher,
            state: state_array,
            index:OUT
        }
    }   
}

impl <'a, T, P, const WIDTH: usize, const OUT: usize> Iterator for StreamCipherIterator<'a, T,P,WIDTH,OUT>
where T:Default+Copy, P: CryptographicPermutation<[T;WIDTH]>
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == OUT {
            self.state = self.cipher.permutation.permute(self.state);
            self.index = 0;
        }
        let res = self.state[self.index];
        self.index += 1;
        Some(res)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{POSEIDON2_PERM, M31StreamCipher};
    use rand::thread_rng;


    // Helper function to check if a matrix is an identity matrix
    fn is_identity_matrix<T>(matrix: &RowMajorMatrix<T>) -> bool
    where
        T: Field + PartialEq + Clone,
    {
        let n = matrix.width();
        if n != matrix.height() {
            return false;
        }

        for i in 0..n {
            for j in 0..n {
                let val = matrix.get(i, j);
                if i == j {
                    if val != T::one() {
                        return false;
                    }
                } else if !val.is_zero() {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn test_invert_matrix() {
        let mut rng = thread_rng();
        let size = 64;

        // Generate a random non-singular matrix
        let matrix = RowMajorMatrix::<Val>::rand(&mut rng, size, size);

        // Invert the matrix
        let inverse = invert_matrix(&matrix);

        // Multiply the matrix by its inverse
        let product = multiply_matrices(&matrix, &inverse);

        // Check if the product is an identity matrix
        assert!(
            is_identity_matrix(&product),
            "Matrix * Inverse is not identity"
        );
    }

    #[test]
    fn test_stream_cipher_iterator() {
        // Create test permutation, similar to POSEIDON2_PERM
        let permutation = POSEIDON2_PERM.clone();
        
        // Create StreamCipher with same type as M31StreamCipher
        let stream = M31StreamCipher::new(permutation);
        
        // Create test seed similar to Hash type
        let test_seed = [Val::new(1), Val::new(2), Val::new(3), Val::new(4)];
        
        // Get first 10 values from iterator
        let values: Vec<Val> = stream.cipher(&test_seed)
            .take(10)
            .collect_vec();
        
        // Verify iterator produces correct number of values
        assert_eq!(values.len(), 10);
        
        // Check that values are non-zero (as this is a cryptographic generator)
        for value in values {
            assert!(!value.is_zero());
        }
        
        // Verify that two consecutive calls produce identical results
        let first_run: Vec<Val> = stream.cipher(&test_seed).take(5).collect_vec();
        let second_run: Vec<Val> = stream.cipher(&test_seed).take(5).collect_vec();
        assert_eq!(first_run, second_run);
    }
}


pub trait CollectVecRational<E:Field> {
    fn collect_vec_rational(self) -> Vec<E>;
}

impl<E:Field, It:IntoIterator<Item=(E,E)>> CollectVecRational<E> for It 
{
    fn collect_vec_rational(self) -> Vec<E> {
        let (num, denom) = self.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
        let denom_inv = batch_multiplicative_inverse(&denom);
        denom_inv.into_iter().zip(num).map(|(denom_inv, num)| denom_inv * num).collect_vec()

    }
}
