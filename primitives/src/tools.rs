use p3_field::{Field, batch_multiplicative_inverse};
use p3_matrix::{Matrix, dense::RowMajorMatrix};
use p3_maybe_rayon::prelude::*;
use core::ops::Deref;
use alloc::vec;
use alloc::vec::Vec;
use itertools::Itertools;

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


#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;
    use p3_mersenne_31::Mersenne31;


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
        let matrix = RowMajorMatrix::<Mersenne31>::rand(&mut rng, size, size);

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
