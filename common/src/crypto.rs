use m31jubjub::{
    eddsa::SigParams,
    hdwallet::priv_key,
    m31::{Fq, Fs, M31JubJubSigParams},
};
use primitives::Val;

const PATH: &str = "m/132120/0'/0'";

pub type Signature = (Fq, Fs);
pub type PublicKey = Fq;
pub type PrivateKey = Fs;

pub fn derive_keys(mnemonic: &str) -> Option<(Fs, Fq)> {
    let sk = priv_key::<M31JubJubSigParams>(mnemonic, PATH).unwrap();
    let pk = M31JubJubSigParams::public_key(sk);
    Some((sk, pk))
}

pub fn sign(message: &[Val], sk: Fs) -> Signature {
    let sig_params = M31JubJubSigParams::default();
    sig_params.sign(message, sk)
}

pub fn hash(message: &[Val]) -> [Val; 8] {
    let sig_params = M31JubJubSigParams::default();
    sig_params.hash_message(message)
}

pub fn verify(message: &[Val], signature: Signature, pk: Fq) -> bool {
    let sig_params = M31JubJubSigParams::default();
    sig_params.verify(&message, signature, pk)
}
