use alloc::vec::Vec;

use crate::curve::{Params, Point, PointProjective};
use p3_symmetric::CryptographicHasher;
use p3_field::PrimeField32;
use ark_ff::{BigInteger, Field, PrimeField};

use num_bigint::BigUint;
use core::iter::successors;
use sha3::{Digest, Keccak256};

use crate::flatten::ExtFieldFlattener;

pub trait SigParams<const HASHER_OUT: usize>: Clone {
    type P: Params;
    type HasherOut;
    type Fb: Field + PrimeField32;
    type Flattener: ExtFieldFlattener<<Self::P as Params>::Fq, Self::Fb>;

    type Hasher: CryptographicHasher<Self::Fb, [Self::Fb; HASHER_OUT]>;

    fn get_hasher(&self) -> &Self::Hasher;

    fn hash_message(&self, message: &[Self::Fb]) -> [Self::Fb; HASHER_OUT] {
        self.get_hasher().hash_slice(message)
    }

    fn hash_r_a_m(
        &self,
        point_r: <Self::P as Params>::Fq,
        point_a: <Self::P as Params>::Fq,
        hashed_message: [Self::Fb; HASHER_OUT],
    ) -> <Self::P as Params>::Fs {
        let t = [point_r, point_a].iter()
            .flat_map(|e| Self::Flattener::flatten(e).into_iter())
            .chain(hashed_message.into_iter())
            .collect::<Vec<_>>();

        let num_limbs = (<Self::P as Params>::Fs::MODULUS_BIT_SIZE + 31) / 32;
        let hash = self.get_hasher().hash_slice(&t);
        let limbs = successors(Some(hash), |x| Some(self.get_hasher().hash_slice(x)))
            .flat_map(|x| x.into_iter())
            .map(|x| x.as_canonical_u32())
            .take(num_limbs as usize)
            .collect::<Vec<_>>();

        BigUint::from_slice(&limbs).into()
    }

    fn hash_secret_m(
        &self,
        secret: <Self::P as Params>::Fs,
        message: &[Self::Fb],
    ) -> <Self::P as Params>::Fs {
        

        let mut hasher = Keccak256::new();
        hasher.update(secret.into_bigint().to_bytes_le());
        for &item in message.iter() {
            hasher.update(item.as_canonical_u32().to_le_bytes());
        }
        let result = hasher.finalize();
        BigUint::from_bytes_le(&result).into()
    }

    // Perform EDDSA signature
    fn sign(
        &self,
        message: &[Self::Fb],
        private_key: <Self::P as Params>::Fs,
    ) -> (<Self::P as Params>::Fq, <Self::P as Params>::Fs) {
        let hashed_message = self.hash_message(message);
        let r = self.hash_secret_m(private_key, &hashed_message);

        let point_r: Point<_> = (Self::P::G8 * r).into();
        let point_a: Point<_> = (Self::P::G8 * private_key).into();
        
        let h = self.hash_r_a_m(point_r.x, point_a.x, hashed_message);
        let s = r + h * private_key;
        (point_r.x, s)
    }

    fn verify(
        &self,
        message: &[Self::Fb],
        signature: (<Self::P as Params>::Fq, <Self::P as Params>::Fs),
        public_key: <Self::P as Params>::Fq,
    ) -> bool {
        let _verify = move || {
            let point_r:PointProjective<_> = Point::subgroup_decompress(signature.0)?.into();
            let point_a:PointProjective<_> = Point::subgroup_decompress(public_key)?.into();
            let hashed_message = self.hash_message(message);
            let h = self.hash_r_a_m(point_r.x, point_a.x, hashed_message);
            let s_g = Self::P::G8 * signature.1;
            let r_plus_ha = point_r + point_a * h;
            Some(s_g == r_plus_ha)
        };

        _verify().unwrap_or(false)
    }
}