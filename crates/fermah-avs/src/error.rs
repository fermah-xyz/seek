#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("fs error: {0}")]
    Fs(#[from] fermah_common::fs::error::Error),
    #[error("keystore file error avs: {0}")]
    KeystoreFile(#[from] fermah_common::crypto::keystore::KeystoreFileError),
    #[error("ethers provider error: {0}")]
    Provider(#[from] ethers::providers::ProviderError),
}
