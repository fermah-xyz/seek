use fermah_common::crypto::signer::{bls::BlsSignerError, ecdsa::EcdsaSignerError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ecdsa signer error: {0}")]
    EcdsaSignerError(#[from] EcdsaSignerError),

    #[error("bls signer error: {0}")]
    BlsSignerError(#[from] BlsSignerError),

    #[error("fs error: {0}")]
    FsError(#[from] fermah_common::fs::error::Error),

    #[error("hex error: {0}")]
    HexError(#[from] const_hex::FromHexError),

    #[error("keystore file error: {0}")]
    KeystoreFile(#[from] fermah_common::crypto::keystore::KeystoreFileError),

    #[error("keystore file exists: {0}")]
    KeystoreExists(String),
}
