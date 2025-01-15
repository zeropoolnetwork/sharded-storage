use hmac::{Hmac, Mac};
use sha2::Sha512;
use core::convert::TryInto;
use alloc::vec::Vec;
use alloc::string::String;

use ark_ff::{PrimeField, biginteger::BigInteger};

use crate::eddsa::SigParams;
use crate::curve::{CurveParams, Point, PointProjective};
use crate::flatten::ExtFieldFlattener;

use bip39::*;
use sha3::{Digest, Keccak256};
use alloc::str::FromStr;
use base58::{FromBase58, ToBase58};



type HmacSha512 = Hmac<Sha512>;
type Seed = [u8; 64];


fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    // Create an HMAC-SHA512 instance
    let mut mac = HmacSha512::new_from_slice(key).expect("HMAC can take key of any size");
    
    // Process input data
    mac.update(data);
    
    // Finalize and get the result as a byte array
    let result = mac.finalize();
    let bytes = result.into_bytes();
    
    // Convert the Vec<u8> into a fixed-size array [u8; 64]
    bytes.as_slice().try_into().expect("HMAC output has incorrect length")
}

pub type ChainCode = [u8; 32];

pub struct ExtendedKey<K> {
    c: ChainCode,
    k:K
}

impl<K> ExtendedKey<K> {
    pub fn new(c: ChainCode, k:K) -> Self {
        Self { c, k }
    }
}


pub fn ckd_priv<P:SigParams>(k: ExtendedKey<<P::P as CurveParams>::Fs>, i: u32) -> ExtendedKey<<P::P as CurveParams>::Fs> {
    
    let mut preimage = [0; 36];
    if i >= 0x80000000 {
        preimage[..32].copy_from_slice(&k.k.into_bigint().to_bytes_be());
    } else {
        let point: Point<_> = (<P::P as CurveParams>::G8 * k.k).into();
        P::Flattener::write_be_bytes_into(&point.x, &mut preimage[..32]);
    }

    preimage[32..].copy_from_slice(&i.to_be_bytes());

    let l = hmac_sha512(&k.c, &preimage);
    let dk: <P::P as CurveParams>::Fs = PrimeField::from_be_bytes_mod_order(&l[..32]);
    let mut ci = [0; 32];
    ci.copy_from_slice(&l[32..]);

    ExtendedKey::new(ci, k.k+dk)
}


pub fn ckd_pub<P:SigParams>(k: ExtendedKey<<P::P as CurveParams>::Fq>, i: u32) -> ExtendedKey<<P::P as CurveParams>::Fq> {
    assert!(i < 0x80000000, "Cannot compute hardened child from public key");
    let mut preimage = [0; 36];
    P::Flattener::write_be_bytes_into(&k.k, &mut preimage[..32]);
    preimage[32..].copy_from_slice(&i.to_be_bytes());

    let l = hmac_sha512(&k.c, &preimage);

    let dk: <P::P as CurveParams>::Fs = PrimeField::from_be_bytes_mod_order(&l[..32]);


    let point = Point::from(
        PointProjective::from(Point::<P::P>::subgroup_decompress(k.k).unwrap()) 
        + <P::P as CurveParams>::G8 * dk
    );

    let ci = l[32..].try_into().unwrap();
    ExtendedKey::new(ci, point.x)
}

pub fn master_key<P:SigParams>(s:Seed) -> ExtendedKey<<P::P as CurveParams>::Fs> {
    let l = hmac_sha512(b"BabyJub seed", &s);
    let k = PrimeField::from_be_bytes_mod_order(&l[..32]);
    let c = l[32..].try_into().unwrap();
    ExtendedKey::new(c, k)
}

pub fn parse_indexes(path: &str) -> Option<Vec<u32>> {
    let steps: Vec<&str> = path.split('/').collect();

    if steps.is_empty() || steps[0] != "m" {
        return None;
    }
    let mut indexes: Vec<u32> = Vec::new();

    for step in steps.iter().skip(1) {
        let is_hardened = step.ends_with('\'');

        let num_str = if is_hardened {
            &step[..step.len() - 1]
        } else {
            step
        };

        let mut index = match num_str.parse::<u32>() {
            Ok(num) => num,
            Err(_) => return None, 
        };

        if index >= 0x80000000 {
            return None;
        }

        if is_hardened {
            index += 0x80000000;
        }

        indexes.push(index);
    }

    Some(indexes)
}


pub fn priv_key<P:SigParams>(mnemonic: &str, path: &str) -> Option<<P::P as CurveParams>::Fs> {
    let seed = Mnemonic::from_str(mnemonic).unwrap().to_seed_normalized("");
    let res = master_key::<P>(seed);
    let extended_key = parse_indexes(path)?.into_iter().fold(res, |acc, i| ckd_priv::<P>(acc, i));
    Some(extended_key.k)
}

pub fn pub_key<P:SigParams>(mnemonic: &str, path: &str) -> Option<<P::P as CurveParams>::Fq> {
    let priv_key = priv_key::<P>(mnemonic, path)?;
    Some(P::public_key(priv_key))
}

fn checksum(data: &[u8]) -> [u8;4] {
    let hash = Keccak256::digest(data);
    hash[..4].try_into().unwrap()
}

pub fn priv_key_to_str<P:SigParams>(k: <P::P as CurveParams>::Fs) -> String {
    let mut bytes = k.into_bigint().to_bytes_le();
    let checksum = checksum(&bytes);
    bytes.extend_from_slice(&checksum);
    bytes.to_base58()
}

pub fn pub_key_to_str<P:SigParams>(k: <P::P as CurveParams>::Fq) -> String {
    let mut bytes = bincode::serialize(&k).unwrap();
    let checksum = checksum(&bytes);
    bytes.extend_from_slice(&checksum);
    bytes.to_base58()
}

pub fn priv_key_from_str<P:SigParams>(s: &str) -> Option<<P::P as CurveParams>::Fs> {
    let bytes = s.from_base58().unwrap();
    if bytes.len() != 36 {
        return None;
    }
    let cs = &bytes[32..];
    if cs != checksum(&bytes[..32]) {
        return None;
    }
    let res:<P::P as CurveParams>::Fs = PrimeField::from_be_bytes_mod_order(&bytes[..32]);
    if res.into_bigint().to_bytes_be() != bytes[..32] {
        return None;
    }

    Some(res)
}

pub fn pub_key_from_str<P:SigParams>(s: &str) -> Option<<P::P as CurveParams>::Fq> {
    let bytes = s.from_base58().unwrap();
    if bytes.len() != 36 {
        return None;
    }
    let cs = &bytes[32..];
    if cs != checksum(&bytes[..32]) {
        return None;
    }
    let res: <P::P as CurveParams>::Fq = bincode::deserialize(&bytes[..32]).ok()?;
    Some(res)
}




#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::m31::M31JubJubSigParams;
    //use libc_print::std_name::println;

    #[test]
    fn test_master_key() {
        let phrase = "must image axis attend cage menu plastic girl outside grab predict matter";

        let pk = priv_key::<M31JubJubSigParams>(phrase, "m/44'/0'/0'/0/0").unwrap();
        

        let pk_str = priv_key_to_str::<M31JubJubSigParams>(pk);

        assert_eq!(pk_str, "141fQtcCBAz94GAGeip4HKJxLJm6ecud47hMxitA1fviK5Bz5");

    }

}