use std::fmt::Debug;

use async_trait::async_trait;
use const_hex::ToHexExt;
use ethers::{
    addressbook::Address,
    core::k256::ecdsa::SigningKey,
    prelude::transaction::{eip2718::TypedTransaction, eip712::Eip712},
    signers::{Signer as EthereumSigner, Wallet, WalletError},
    types::{Signature, SignatureError, H256},
};
use k256::{ecdsa::VerifyingKey, elliptic_curve::group::GroupEncoding};
use rand_core::CryptoRngCore;

use crate::{
    crypto::signer::Signer,
    hash::{
        blake3::{Blake3Hash, Blake3Hasher},
        Hashable,
    },
};

#[derive(thiserror::Error, Debug)]
pub enum EcdsaSignerError {
    #[error("eth wallet error: {0}")]
    Wallet(#[from] WalletError),
    #[error("eth signature error: {0}")]
    Signature(#[from] SignatureError),
    #[error("eth ecdsa error: {0}")]
    Ecdsa(#[from] k256::ecdsa::Error),
    #[error("eth pkcs8 error: {0}")]
    Pkcs8(#[from] k256::pkcs8::Error),
    #[error("eth pkcs8 pki error: {0}")]
    Pkcs8Pki(#[from] k256::pkcs8::spki::Error),
    #[error("hex error: {0}")]
    FromHex(#[from] const_hex::FromHexError),
}

/// An Ethereum ECDSA private-public key pair which can be used for signing messages.
/// This signer uses only the last 20 bytes of the public key as the verfying key.
#[derive(Clone)]
pub struct EcdsaSigner {
    wallet: Wallet<SigningKey>,
    public_key: VerifyingKey,
}

impl PartialEq<Self> for EcdsaSigner {
    fn eq(&self, other: &Self) -> bool {
        self.public_key == other.public_key
    }
}

impl Debug for EcdsaSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "EthSigner {{ address: {:?} }}",
            self.verifying_key().encode_hex_with_prefix()
        ))
    }
}

impl Signer for EcdsaSigner {
    type PrivateKey = SigningKey;
    type PublicKey = VerifyingKey;
    type VerifyingKey = Address;
    type Signature = Signature;
    type Hasher = Blake3Hasher;
    type Hash = Blake3Hash;
    type SignerError = EcdsaSignerError;

    fn from_key(key: Self::PrivateKey) -> Self {
        let verifying_key = *key.verifying_key();
        let wallet = Wallet::from(key);
        EcdsaSigner {
            public_key: verifying_key,
            wallet,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::SignerError>
    where
        Self: Sized,
    {
        let private_key = SigningKey::from_bytes(bytes.into())?;
        Ok(Self::from_key(private_key))
    }

    fn from_random(mut rng: impl CryptoRngCore) -> Result<(Self, Vec<u8>), Self::SignerError>
    where
        Self: Sized,
    {
        let private_key = SigningKey::random(&mut rng);
        Ok((
            Self::from_key(private_key.clone()),
            private_key.to_bytes().to_vec(),
        ))
    }

    fn hash_and_sign<D: Hashable>(&self, data: D) -> Result<Self::Signature, Self::SignerError> {
        let data_hash = H256::from_slice(data.hash::<Self::Hasher>().as_ref());
        Ok(self.wallet.sign_hash(data_hash)?)
    }

    fn sign(&self, data: &[u8]) -> Result<Self::Signature, Self::SignerError> {
        let data_hash = H256::from_slice(data);
        Ok(self.wallet.sign_hash(data_hash)?)
    }

    fn public_key_vec(&self) -> Vec<u8> {
        self.public_key().as_affine().to_bytes().to_vec()
    }

    fn verifying_key_vec(&self) -> Vec<u8> {
        self.verifying_key().as_bytes().to_vec()
    }

    fn public_key(&self) -> Self::PublicKey {
        self.public_key
    }

    fn verifying_key(&self) -> Self::VerifyingKey {
        self.wallet.address()
    }

    fn verify(
        hash: &Self::Hash,
        pubkey: &Self::VerifyingKey,
        signature: &Self::Signature,
    ) -> Result<(), Self::SignerError> {
        let hash = H256::from_slice(hash.as_ref());
        signature.verify(hash, *pubkey).map_err(|e| e.into())
    }
}

#[async_trait]
impl ethers::signers::Signer for EcdsaSigner {
    type Error = WalletError;

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        self.wallet.sign_message(message).await
    }

    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, Self::Error> {
        self.wallet.sign_transaction(tx).await
    }

    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, Self::Error> {
        self.wallet.sign_typed_data(payload).await
    }

    fn address(&self) -> Address {
        self.wallet.address()
    }

    fn chain_id(&self) -> u64 {
        self.wallet.chain_id()
    }

    fn with_chain_id<T: Into<u64>>(mut self, chain_id: T) -> Self {
        self.wallet = self.wallet.with_chain_id(chain_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use ethers::types::H160;
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;
    use crate::{crypto::signer::Signer, hash::blake3::Blake3Hasher};

    struct TestHashable {
        data: String,
    }

    impl Hashable for TestHashable {
        fn collect(&self) -> std::borrow::Cow<[u8]> {
            self.data.as_bytes().into()
        }
    }

    #[test]
    fn test_sign() {
        let key = SigningKey::random(&mut StdRng::seed_from_u64(0));
        let signer = EcdsaSigner::from_key(key);
        let data = TestHashable {
            data: "test".to_string(),
        };

        let hash = data.hash::<Blake3Hasher>();
        let signature = signer.hash_and_sign(data).unwrap();
        assert!(EcdsaSigner::verify(
            &hash,
            &H160::from_slice(signer.verifying_key_vec().as_slice()),
            &signature
        )
        .is_ok());

        assert_eq!(format!("{}", signature), "9db6e894c27b4a3b50a3cd3142f2d0a0b7c6c674f1624144a5a842d7cc2d43865a303aafffe5d0144bd9621bd3fb565387c3a18c25e2c812d93a1652fe627b001b");
    }
}
