use rand_core::CryptoRngCore;

pub mod scrypt;

pub trait Kdf: Sized {
    const NAME: &'static str;

    fn fast(rng: impl CryptoRngCore) -> Self;
    fn secure(rng: impl CryptoRngCore) -> Self;

    type Error: std::error::Error;
    type Params;

    fn new(params: Self::Params) -> Self;

    fn derive_key(&self, password: &[u8], out: &mut [u8]) -> Result<(), Self::Error>;
}
