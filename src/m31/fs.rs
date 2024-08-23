use ark_ff::{Fp256, MontBackend, MontConfig};

#[derive(MontConfig)]
#[modulus = "56539105862283715552830147462192958429703948449175675457269070040950777139"]
#[generator = "2"]
pub struct FsConfig;

pub type Fs = Fp256<MontBackend<FsConfig, 4>>;