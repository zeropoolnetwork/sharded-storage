use p3_field::extension::{BinomialExtensionField, Complex};
use p3_mersenne_31::Mersenne31;

pub type FqBase = Mersenne31;
pub type Fq = BinomialExtensionField<Complex<Mersenne31>, 4>;
pub type Fs = crate::m31::fs::Fs;


// build quadratic complex extension field from base field array
// F = x_0 + x_1 * i + x_2 * j + x_3 * ij + x_4 * j^2 + x_5 * ij^2 + x_6 * j^3 + x_7 * ij^3
// where i^2 = j and j^4 = i + 2
pub const fn fq_new_from_raw(raw: [u32; 8]) -> Fq {
    Fq::from_raw([
        Complex::new(
            Mersenne31::new(raw[0]),
            Mersenne31::new(raw[1]),
        ),
        Complex::new(
            Mersenne31::new(raw[2]),
            Mersenne31::new(raw[3]),
        ),
        Complex::new(
            Mersenne31::new(raw[4]),
            Mersenne31::new(raw[5]),
        ),
        Complex::new(
            Mersenne31::new(raw[6]),
            Mersenne31::new(raw[7]),
        ),
    ])
}