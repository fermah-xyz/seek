use aes::Aes128;
use const_hex::ToHexExt;
use ctr::{
    cipher::{KeyIvInit, StreamCipher, StreamCipherCoreWrapper},
    flavors,
    CtrCore,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tracing::warn;
use zeroize::ZeroizeOnDrop;

use crate::{
    crypto::{cipher::Cipher, kdf::Kdf},
    hash::{keccak256::Keccak256Hasher, Hasher},
    serialization::encoding::hex_encoded_no_prefix,
};

const AES128CTR_KEY_LEN: usize = 16;
const AES128CTR_KDF_LEN: usize = AES128CTR_KEY_LEN * 2;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct Aes128Params {
    #[serde(with = "hex_encoded_no_prefix")]
    pub iv: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum Aes128CtrCipherError {
    #[error("kdf error: {0}")]
    Kdf(String),

    #[error("mac mismatch: {expected} != {found}, wrong password!")]
    MacMismatch { expected: String, found: String },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, ZeroizeOnDrop)]
pub struct Aes128CtrCipher<KDF: Kdf> {
    #[zeroize(skip)]
    #[serde(rename = "cipher")]
    name: String,

    #[zeroize(skip)]
    #[serde(rename = "cipherparams")]
    pub params: Aes128Params,

    #[zeroize(skip)]
    #[serde(rename = "kdf")]
    kdf_name: String,

    #[zeroize(skip)]
    #[serde(rename = "kdfparams")]
    kdf: KDF,

    #[serde(rename = "ciphertext", with = "hex_encoded_no_prefix")]
    pub data: Vec<u8>,

    #[zeroize(skip)]
    #[serde(rename = "mac", with = "hex_encoded_no_prefix")]
    pub mac: Vec<u8>,
}

impl<KDF: Kdf> Aes128CtrCipher<KDF> {
    pub fn new(data: Vec<u8>, params: Aes128Params, kdf: KDF) -> Self {
        Self {
            name: Self::NAME.to_string(),
            params,
            kdf_name: KDF::NAME.to_string(),
            kdf,
            data,
            mac: vec![],
        }
    }

    pub fn from_data(data: Vec<u8>, fast: bool) -> Self {
        let mut iv = [0u8; AES128CTR_KEY_LEN];
        OsRng.fill_bytes(&mut iv);

        let kdf = fast
            .then(|| {
                warn!("cipher KDF fast mode enabled! this is insecure, do not use in production");
                KDF::fast(&mut OsRng)
            })
            .unwrap_or_else(|| KDF::secure(&mut OsRng));

        Self {
            name: Self::NAME.to_string(),
            params: Aes128Params { iv: iv.to_vec() },
            kdf_name: KDF::NAME.to_string(),
            kdf,
            data,
            mac: vec![],
        }
    }

    fn derive_key(
        &mut self,
        password: &[u8],
    ) -> Result<[u8; AES128CTR_KDF_LEN], Aes128CtrCipherError> {
        let mut key = [0u8; AES128CTR_KDF_LEN];
        self.kdf
            .derive_key(password, &mut key)
            .map_err(|e| Aes128CtrCipherError::Kdf(format!("{}", e)))?;

        Ok(key)
    }

    fn apply_xor(iv: &[u8], key: [u8; AES128CTR_KEY_LEN], data: &mut [u8]) {
        let iv: [u8; AES128CTR_KEY_LEN] = iv.try_into().unwrap();

        let mut cipher = <Aes128CtrCipher<KDF> as Cipher>::CoreCipher::new(&key.into(), &iv.into());
        cipher.apply_keystream(data);
    }
}

impl<KDF: Kdf> Cipher for Aes128CtrCipher<KDF> {
    const NAME: &'static str = "aes-128-ctr";

    type Error = Aes128CtrCipherError;
    type CoreCipher = StreamCipherCoreWrapper<CtrCore<Self::BlockCipher, flavors::Ctr128LE>>;
    type BlockCipher = Aes128;
    type Kdf = KDF;
    type KdfParams = KDF::Params;

    fn data_len(&self) -> usize {
        self.data.len()
    }

    fn encrypt(&mut self, password: &[u8]) -> Result<(), Self::Error> {
        let cipher_key = self.derive_key(password)?;
        Self::apply_xor(
            self.params.iv.as_slice(),
            cipher_key[..AES128CTR_KEY_LEN].try_into().unwrap(),
            self.data.as_mut_slice(),
        );

        let mut hasher = Keccak256Hasher::new();
        hasher.update(
            &[
                &cipher_key[AES128CTR_KEY_LEN..AES128CTR_KDF_LEN],
                self.data.as_slice(),
            ]
            .concat(),
        );

        self.mac = hasher.finalize().as_ref().to_vec();
        Ok(())
    }

    fn decrypt(&mut self, password: &[u8]) -> Result<&Self, Self::Error> {
        let cipher_key = self.derive_key(password)?;

        let mut hasher = Keccak256Hasher::new();
        hasher.update(
            &[
                &cipher_key[AES128CTR_KEY_LEN..AES128CTR_KDF_LEN],
                self.data.as_slice(),
            ]
            .concat(),
        );
        let mac = hasher.finalize().as_ref().to_vec();

        Self::apply_xor(
            self.params.iv.as_slice(),
            cipher_key[..AES128CTR_KEY_LEN].try_into().unwrap(),
            self.data.as_mut_slice(),
        );

        if self.mac != mac {
            return Err(Aes128CtrCipherError::MacMismatch {
                expected: self.mac.encode_hex_with_prefix(),
                found: mac.encode_hex_with_prefix(),
            });
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use const_hex::ToHexExt;
    use rand::prelude::StdRng;
    use rand_core::{RngCore, SeedableRng};

    use super::*;
    use crate::crypto::kdf::scrypt::ScryptKdf;

    #[test]
    fn test_aes128ctr_cipher_encrypt() {
        let mut iv = [0u8; 16];
        StdRng::seed_from_u64(0).fill_bytes(&mut iv);

        let data = [1u8; 16];
        println!("data: {}", data.encode_hex_with_prefix());

        let mut cipher = Aes128CtrCipher::new(
            data.to_vec(),
            Aes128Params { iv: iv.to_vec() },
            ScryptKdf::fast(StdRng::seed_from_u64(0)),
        );
        let password = "password";

        cipher.encrypt(password.as_bytes()).unwrap();

        assert_eq!(
            cipher.params.iv.encode_hex_with_prefix(),
            "0x7f6f2ccdb23f2abb7b69278e947c01c6".to_string()
        );

        assert_eq!(
            cipher.mac.encode_hex_with_prefix(),
            "0x63748fdab0603b17be14ed1b753e5a00588cf2ad684bc899bb17ab74cf8fc95a".to_string()
        );

        assert_eq!(
            cipher.data.encode_hex_with_prefix(),
            "0x898535671969a137fcd2c5194018ca95".to_string()
        );

        cipher.decrypt(password.as_bytes()).unwrap();

        assert_eq!(
            cipher.data.encode_hex_with_prefix(),
            "0x01010101010101010101010101010101".to_string()
        );
    }
}
