pub mod aes128ctr;
pub mod plain;

use aes::cipher::{BlockCipher, BlockEncryptMut};
use ctr::cipher::StreamCipher;

use crate::crypto::kdf::Kdf;

pub trait Cipher {
    const NAME: &'static str;

    type Error: std::error::Error;

    type CoreCipher: StreamCipher;
    type BlockCipher: BlockEncryptMut + BlockCipher;
    type Kdf: Kdf<Params = Self::KdfParams>;
    type KdfParams;

    fn data_len(&self) -> usize;

    fn encrypt(&mut self, password: &[u8]) -> Result<(), Self::Error>;

    fn decrypt(&mut self, password: &[u8]) -> Result<&Self, Self::Error>;
}
