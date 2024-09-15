use std::{
    env::VarError,
    fmt::Debug,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use tracing::{debug, warn};

use crate::fs::error::Error;

pub mod error;
pub mod hash;
pub mod json;
pub mod mountable;
pub mod rand;

pub const FERMAH_CONFIG_ENV_VAR: &str = "FERMAH_CONFIG";
const DEFAULT_HOME_DIR_BASE: &str = ".fermah";

/// Make sure that a directory according to given path exist.
pub async fn ensure_dir<P: AsRef<Path>>(p: P, perms: Option<u32>) -> Result<(), std::io::Error> {
    let path = p.as_ref();
    tracing::debug!("ensuring dir exists: {}", path.display());
    if !path.exists() {
        tokio::fs::create_dir_all(path).await?;

        if let Some(perms) = perms {
            tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(perms)).await?;
        }
    }
    Ok(())
}

/// Make sure that a directory according to given path exist. Synchronous version of `ensure_dir`.
pub fn ensure_dir_sync<P: AsRef<Path>>(p: P, perms: Option<u32>) -> Result<(), std::io::Error> {
    let path = p.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path)?;

        if let Some(perms) = perms {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(perms))?;
        }
    }
    Ok(())
}

/// Returns App's home directory, which is typically at `~/.fermah`
pub async fn app_home_dir() -> Result<PathBuf, Error> {
    let base = match std::env::var(FERMAH_CONFIG_ENV_VAR) {
        Ok(path) => path.into(),
        Err(_) => {
            home::home_dir()
                .ok_or(Error::InvalidHomeDir)?
                .join(DEFAULT_HOME_DIR_BASE)
        }
    };
    debug!("config files directory: {}", base.display());
    ensure_dir(&base, None).await?;
    Ok(base)
}

/// Returns App's home directory, which is typically at `~/.fermah`. Synchronous version of `app_home_dir`.
pub fn app_home_dir_sync() -> Result<PathBuf, Error> {
    let base = match std::env::var(FERMAH_CONFIG_ENV_VAR) {
        Ok(path) => path.into(),
        Err(_) => {
            home::home_dir()
                .ok_or(Error::InvalidHomeDir)?
                .join(DEFAULT_HOME_DIR_BASE)
        }
    };
    debug!("config files directory: {}", base.display());
    ensure_dir_sync(&base, None)?;
    Ok(base)
}

pub async fn dir_from_env<E: AsRef<std::ffi::OsStr> + Debug, D: AsRef<Path>>(
    env: E,
    dir: D,
    default: PathBuf,
) -> Result<PathBuf, std::io::Error> {
    let root = match std::env::var(&env) {
        Ok(directory) => directory.into(),
        Err(e) => {
            let default = default.join(dir);
            if let VarError::NotUnicode(contents) = e {
                warn!(
                    ?env,
                    ?contents,
                    ?default,
                    "env var value is not unicode, using default"
                );
            }
            default
        }
    };

    ensure_dir(&root, None).await?;
    Ok(root)
}

pub async fn copy_dir(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if !src.exists() {
        return Ok(());
    }

    ensure_dir(&dst, None).await?;

    let mut dir_str = tokio::fs::read_dir(src).await?;

    while let Some(entry) = dir_str.next_entry().await? {
        let path = entry.path();
        println!(
            "Copying {:?} to {:?}",
            path,
            &dst.join(path.file_name().unwrap())
        );

        if path.is_file() {
            tokio::fs::copy(&path, &dst.join(path.file_name().unwrap())).await?;
        }
    }

    Ok(())
}
