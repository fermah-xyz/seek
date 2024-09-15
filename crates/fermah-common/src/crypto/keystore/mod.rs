use std::env;

use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tokio::io;
use uuid::Uuid;
use zeroize::ZeroizeOnDrop;

use crate::{
    cli,
    cli::spinner::{Spinner, SpinnerTemplate},
    crypto::{
        cipher::{aes128ctr::Aes128CtrCipher, plain::PlainCipher, Cipher},
        kdf::scrypt::ScryptKdf,
        signer::Signer,
    },
    fs::json::Json,
    serialization::encoding::hex_encoded_no_prefix,
};

pub const KEYS_DIR: &str = "keys";
pub const KEYSTORE_PASS_ENV: &str = "FERMAH_KEYSTORE_PW_FILE";

pub trait Keystore {
    type Data: Cipher + ZeroizeOnDrop;
}

#[derive(thiserror::Error, Debug)]
pub enum KeystoreFileError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("bls signer error: {0}")]
    BlsSigner(#[from] crate::crypto::signer::bls::BlsSignerError),

    #[error("ecdsa signer error: {0}")]
    EcdsaSigner(#[from] crate::crypto::signer::ecdsa::EcdsaSignerError),

    #[error("aes128ctr cipher error: {0}")]
    Aes128CtrError(#[from] crate::crypto::cipher::aes128ctr::Aes128CtrCipherError),

    #[error("unable to find password in stdin or env var")]
    Password,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, ZeroizeOnDrop)]
#[serde(rename_all = "lowercase")]
pub enum KeystoreFile {
    Plain(KeystoreCipher<PlainCipher>),

    #[zeroize(skip)]
    Encrypted(KeystoreCipher<Aes128CtrCipher<ScryptKdf>>),
}

impl KeystoreFile {
    pub async fn from_config(config: &KeystoreConfig) -> Result<Self, crate::fs::error::Error> {
        let path = crate::fs::app_home_dir()
            .await?
            .join(KEYS_DIR)
            .join(format!("{}.json", config.key));
        Self::from_json_path(path).await
    }

    pub async fn get_password(name: &str) -> Result<String, KeystoreFileError> {
        let password = env::var(KEYSTORE_PASS_ENV).ok();

        match password {
            Some(pw_file) => {
                let pw = tokio::fs::read_to_string(pw_file).await?;
                Ok(pw.trim().to_string())
            }
            None => {
                let password = cli::prompts::prompt_for_password_unlock(name)?;
                Ok(password.trim().to_string())
            }
        }
    }

    pub async fn to_signer<S: Signer>(&mut self) -> Result<S, KeystoreFileError>
    where
        KeystoreFileError: From<<S as Signer>::SignerError>,
    {
        match self {
            KeystoreFile::Plain(plain) => Ok(S::from_bytes(plain.crypto.data.as_slice())?),
            KeystoreFile::Encrypted(ref mut enc) => {
                let password = Self::get_password(&enc.name).await?;

                let spinner = Spinner::new(1, "Decrypting", SpinnerTemplate::Default);
                let decrypted = enc.crypto.decrypt(password.as_bytes())?;
                spinner.finish("Unlocked keystore!", true);

                Ok(S::from_bytes(decrypted.data.as_slice())?)
            }
        }
    }
}

#[serde_as]
#[derive(Parser, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KeystoreConfig {
    /// Name of the keystore
    #[arg(long, default_value = "default")]
    pub key: String,
    // There is no support in clap for enum unit variants yet,
    // otherwise KeystoreLocation would go here
}

/// Encrypted keystore as defined in:
/// https://ethereum.org/en/developers/docs/data-structures-and-encoding/web3-secret-storage/
#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, ZeroizeOnDrop)]
pub struct KeystoreCipher<C: Cipher + ZeroizeOnDrop> {
    pub crypto: C,

    #[serde(with = "hex_encoded_no_prefix")]
    #[zeroize(skip)]
    pub address: Vec<u8>,

    #[serde_as(as = "DisplayFromStr")]
    #[zeroize(skip)]
    pub id: Uuid,

    #[zeroize(skip)]
    pub name: String,

    #[zeroize(skip)]
    pub version: u8,
}

impl<C: Cipher + ZeroizeOnDrop> KeystoreCipher<C> {
    pub fn new(crypto: C, address: Vec<u8>, uuid: Uuid, name: String) -> Self {
        Self {
            crypto,
            address,
            id: uuid,
            name,
            version: 3,
        }
    }
}

impl<C: Cipher + ZeroizeOnDrop> Keystore for KeystoreCipher<C> {
    type Data = C;
}

#[cfg(test)]
mod tests {
    use rand::prelude::StdRng;
    use rand_core::{RngCore, SeedableRng};

    use super::*;
    use crate::crypto::{
        cipher::aes128ctr::{Aes128CtrCipher, Aes128Params},
        kdf::{scrypt::ScryptKdf, Kdf},
    };

    #[test]
    fn test_keystore_v3() {
        let mut iv = [0u8; 16];
        StdRng::seed_from_u64(0).fill_bytes(&mut iv);

        let data = [1u8; 16];
        let kdf = ScryptKdf::fast(&mut StdRng::seed_from_u64(0));
        let mut cipher = Aes128CtrCipher::new(data.to_vec(), Aes128Params { iv: iv.to_vec() }, kdf);

        cipher.encrypt("password".as_bytes()).unwrap();

        let keystore = KeystoreFile::Encrypted(KeystoreCipher::new(
            cipher,
            vec![],
            Uuid::new_v4(),
            "test".to_string(),
        ));

        let serialized = serde_json::to_string_pretty(&keystore).unwrap();

        println!("{}", serialized);

        let deserialized: KeystoreFile = serde_json::from_str(&serialized).unwrap();

        match (keystore, deserialized) {
            (KeystoreFile::Encrypted(ref keystore), KeystoreFile::Encrypted(ref deserialized)) => {
                assert_eq!(keystore.crypto.data, deserialized.crypto.data);
            }
            _ => panic!("Keystore types do not match"),
        }
    }
}
