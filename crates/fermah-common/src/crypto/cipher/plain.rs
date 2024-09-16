use aes::Aes128;
use cipher::StreamCipherCoreWrapper;
use ctr::{flavors, CtrCore};
use serde::{Deserialize, Serialize};
use zeroize::ZeroizeOnDrop;

use crate::{
    crypto::{
        cipher::Cipher,
        kdf::scrypt::{ScryptKdf, ScryptKdfParams},
    },
    serialization::encoding::hex_encoded_no_prefix,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, ZeroizeOnDrop)]
pub struct PlainCipher {
    #[serde(rename = "ciphertext", with = "hex_encoded_no_prefix")]
    pub data: Vec<u8>,
}

impl Cipher for PlainCipher {
    const NAME: &'static str = "plain";

    type Error = std::io::Error;
    type CoreCipher = StreamCipherCoreWrapper<CtrCore<Self::BlockCipher, flavors::Ctr128LE>>;
    type BlockCipher = Aes128;
    type Kdf = ScryptKdf;
    type KdfParams = ScryptKdfParams;

    fn data_len(&self) -> usize {
        self.data.len()
    }

    fn encrypt(&mut self, _password: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn decrypt(&mut self, _password: &[u8]) -> Result<&Self, Self::Error> {
        Ok(self)
    }
}

impl PlainCipher {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
