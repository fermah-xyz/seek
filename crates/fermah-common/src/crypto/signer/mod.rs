pub mod bls;
pub mod ecdsa;

use std::fmt::Debug;

use clap::ValueEnum;
use const_hex::{traits::FromHex, FromHexError, ToHexExt};
use rand_core::CryptoRngCore;
use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    hash::{Hashable, Hasher},
    serialization::encoding::hex_encoded,
};

#[derive(Serialize, Deserialize, Display, ValueEnum, Debug, Clone)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SignerType {
    Ecdsa,
    BLS,
}

/// Generic trait for signing some hashable data.
pub trait Signer {
    type PrivateKey;
    type PublicKey;
    type VerifyingKey;
    type Signature;
    type Hasher: Hasher<Hash = Self::Hash>;
    type Hash: ToHexExt + FromHex<Error = FromHexError> + AsRef<[u8]>;
    type SignerError;

    fn from_key(key: Self::PrivateKey) -> Self;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::SignerError>
    where
        Self: Sized;

    fn from_random(rng: impl CryptoRngCore) -> Result<(Self, Vec<u8>), Self::SignerError>
    where
        Self: Sized;

    fn hash_and_sign<D: Hashable>(&self, data: D) -> Result<Self::Signature, Self::SignerError>;

    fn sign(&self, data: &[u8]) -> Result<Self::Signature, Self::SignerError>;

    fn public_key(&self) -> Self::PublicKey;
    fn verifying_key(&self) -> Self::VerifyingKey;

    fn public_address(&self) -> Vec<u8>;

    fn verify(
        hash: &Self::Hash,
        pubkey: &Self::VerifyingKey,
        signature: &Self::Signature,
    ) -> Result<(), Self::SignerError>;
}

/// Container that holds a payload and signature
#[derive(Serialize, Deserialize, Hash, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignedData<D: Serialize + Hashable + Clone, S: Signer> {
    #[serde(with = "hex_encoded")]
    pub hash: S::Hash,

    pub payload: D,
    pub public_key: S::VerifyingKey,
    pub signature: S::Signature,
}

impl<D: Serialize + Hashable + Clone, S: Signer> Hashable for SignedData<D, S>
where
    S::VerifyingKey: Clone + AsRef<[u8]>,
    S::Signature: Clone + AsRef<[u8]>,
{
    fn collect(&self) -> std::borrow::Cow<[u8]> {
        [
            self.public_key.clone().as_ref(),
            self.signature.clone().as_ref(),
        ]
        .concat()
        .into()
    }
}

#[allow(dead_code)]
impl<D: Serialize + Hashable + Clone, S: Signer> SignedData<D, S> {
    pub fn new(payload: D, signer: &S) -> Result<Self, S::SignerError> {
        let hash = payload.hash::<S::Hasher>();
        let signature = signer.hash_and_sign(payload.clone())?;

        Ok(SignedData {
            payload,
            hash,
            public_key: signer.verifying_key(),
            signature,
        })
    }

    pub fn verify(&self) -> Result<(), S::SignerError> {
        S::verify(&self.hash, &self.public_key, &self.signature)
    }
}

#[cfg(test)]
mod tests {
    use rand::{prelude::StdRng, SeedableRng};

    use super::*;
    use crate::crypto::signer::ecdsa::EcdsaSigner;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct TestData {
        data: String,
    }

    impl Hashable for TestData {
        fn collect(&self) -> std::borrow::Cow<[u8]> {
            self.data.as_bytes().into()
        }
    }

    #[tokio::test]
    async fn test_signed_data() {
        let (signer, _) = EcdsaSigner::from_random(&mut StdRng::seed_from_u64(0)).unwrap();

        let data = TestData {
            data: "test".to_string(),
        };

        let signed_data = SignedData::new(data, &signer).unwrap();

        assert!(signed_data.verify().is_ok());
        assert_eq!(format!("{}", signed_data.signature), "9db6e894c27b4a3b50a3cd3142f2d0a0b7c6c674f1624144a5a842d7cc2d43865a303aafffe5d0144bd9621bd3fb565387c3a18c25e2c812d93a1652fe627b001b");
    }
}
