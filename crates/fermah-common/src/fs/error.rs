use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("invalid home dir")]
    InvalidHomeDir,
    #[error("invalid file name")]
    InvalidFileName,
}
