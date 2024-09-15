use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed on bytes conversion: {0}")]
    BytesConversion(#[from] bincode::Error),
}
