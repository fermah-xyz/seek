use crate::ProfileKey;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde_json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("fs error: {0}")]
    Fs(#[from] fermah_common::fs::error::Error),
    #[error("profile {profile:?}, template: {template:?} not found at {dir}")]
    ProfileNotFound {
        profile: ProfileKey,
        template: Option<String>,
        dir: std::path::PathBuf,
    },
    #[error("encountered an invalid path: {0}")]
    InvalidPath(std::path::PathBuf),
    #[error("encountered a non UTF8 path: {0}")]
    NonUtf8Path(std::path::PathBuf),
    #[error("failed to merge config for profile: {profile:?}")]
    Merge { profile: ProfileKey },
}
