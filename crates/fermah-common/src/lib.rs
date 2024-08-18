use std::path::PathBuf;

pub mod cli;
pub mod crypto;
pub mod executable;
pub mod fs;
pub mod hash;
pub mod http;
pub mod manifest;
pub mod operator;
pub mod proof;
pub mod releaser;
pub mod resource;
pub mod resources;
pub mod serialization;
pub mod types;
pub mod vec;

/// Root directory to write sled databases to.
const FERMAH_DB_ROOT_PATH_ENV_VAR: &str = "FERMAH_DB_ROOT_PATH";

pub fn database_path(db_name: &str) -> PathBuf {
    let db_root_path = std::env::var(FERMAH_DB_ROOT_PATH_ENV_VAR).unwrap_or_default();
    let db_path = PathBuf::from(db_root_path).join(db_name);
    tracing::debug!("opening DB: {}", db_path.display());
    db_path
}
