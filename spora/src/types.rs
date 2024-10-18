use p3_field::{AbstractField, PrimeField32};
use primitives::Val;


#[derive(Clone, Copy, Debug)]
pub struct Nonce(u64);


// assumes x < M31^2
fn div_mod_mersenne31(x: u64) -> (u32, u32) {
    const M31: u64 = Val::ORDER_U32 as u64;
    let t = (x & M31) + (x >> 31);
    if t <= M31 {
        ((x >> 31) as u32, t as u32)
    } else {
        (((x >> 31) + 1) as u32, (t - M31) as u32)
    }
}


impl Nonce {
    pub fn new(nonce:u64) -> Self {
        const M31_2: u64 = (Val::ORDER_U32 as u64)*(Val::ORDER_U32 as u64);
        assert!(nonce < M31_2, "Nonce overflow");
        Self(nonce)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_mersenne_31_word(&self) -> [Val; 2] {
        let (hi, lo) = div_mod_mersenne31(self.0);
        [Val::from_canonical_u32(lo), Val::from_canonical_u32(hi)]
    }
}


