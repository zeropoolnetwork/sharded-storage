use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use m31jubjub::{
    eddsa::SigParams,
    hdwallet::priv_key,
    m31::{Fq, Fs, M31JubJubSigParams},
};
use serde::{Deserialize, Serialize};
use primitives::Val;
use serde_with::{DeserializeAs, SerializeAs};

const PATH: &str = "m/132120/0'/0'";

#[serde_with::serde_as]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Signature(Fq, #[serde_as(as = "AsArk")] Fs);


impl From<(Fq, Fs)> for Signature {
    fn from((r, s): (Fq, Fs)) -> Signature {
        Signature(r, s)
    }
}

impl From<Signature> for (Fq, Fs) {
    fn from(sig: Signature) -> (Fq, Fs) {
        (sig.0, sig.1)
    }
}

pub struct AsArk;

impl<T> SerializeAs<T> for AsArk
where
    T: CanonicalSerialize,
{
    fn serialize_as<S>(val: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = vec![];
        val.serialize_compressed(&mut bytes)
            .map_err(serde::ser::Error::custom)?;

        serde_with::Bytes::serialize_as(&bytes, serializer)
    }
}

impl<'de, T> DeserializeAs<'de, T> for AsArk
where
    T: CanonicalDeserialize,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde_with::Bytes::deserialize_as(deserializer)?;
        T::deserialize_compressed(&mut &bytes[..]).map_err(serde::de::Error::custom)
    }
}
pub type PublicKey = Fq;
pub type PrivateKey = Fs;

pub fn derive_keys(mnemonic: &str) -> Option<(Fs, Fq)> {
    let sk = priv_key::<M31JubJubSigParams>(mnemonic, PATH).unwrap();
    let pk = M31JubJubSigParams::public_key(sk);
    Some((sk, pk))
}

pub fn sign(message: &[Val], sk: Fs) -> Signature {
    let sig_params = M31JubJubSigParams::default();
    sig_params.sign(message, sk).into()
}

pub fn hash(message: &[Val]) -> [Val; 8] {
    let sig_params = M31JubJubSigParams::default();
    sig_params.hash_message(message)
}

pub fn verify(message: &[Val], signature: Signature, pk: Fq) -> bool {
    let sig_params = M31JubJubSigParams::default();
    sig_params.verify(&message, signature.into(), pk)
}
