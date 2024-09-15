use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::Serialize;

use super::{app_home_dir, app_home_dir_sync, ensure_dir_sync};
use crate::fs::{ensure_dir, error::Error};

pub const FERMAH_HOST_CONFIG_ENV_VAR: &str = "FERMAH_HOST_CONFIG";
const ASSETS_DIRECTORY: &str = "assets";

/// Returns App's home directory at host machine in case the operator is run in a docker container.
/// Returns `None` if FERMAH_HOST_CONFIG env variable is not set.
pub async fn app_home_dir_at_host() -> Result<Option<PathBuf>, Error> {
    if let Ok(path) = std::env::var(FERMAH_HOST_CONFIG_ENV_VAR) {
        let base: PathBuf = path.into();
        ensure_dir(&base, None).await?;
        Ok(Some(base))
    } else {
        Ok(None)
    }
}

/// Returns App's home directory at host machine in case the operator is run in a docker container.
/// Returns `None` if FERMAH_HOST_CONFIG env variable is not set. Sync version of `app_home_dir_at_host` function.
pub fn app_home_dir_at_host_sync() -> Result<Option<PathBuf>, Error> {
    if let Ok(path) = std::env::var(FERMAH_HOST_CONFIG_ENV_VAR) {
        let base: PathBuf = path.into();
        ensure_dir_sync(&base, None)?;
        Ok(Some(base))
    } else {
        Ok(None)
    }
}

/// Represents some location that is accessible locally, and if this location is mounted (example operator is dockerized),
/// shows the same location at the host machine.
#[derive(Debug, Clone, Hash, PartialEq, Serialize)]
pub struct PathBufMountable {
    local: PathBuf,
    host: Option<PathBuf>,
}

impl PathBufMountable {
    pub fn new(local: PathBuf, host: Option<PathBuf>) -> Self {
        Self { local, host }
    }

    pub fn local(&self) -> &Path {
        &self.local
    }

    pub fn at_host(&self) -> &Path {
        self.host.as_ref().unwrap_or(&self.local)
    }
}

impl From<PathBufMirror> for PathBufMountable {
    fn from(value: PathBufMirror) -> Self {
        // TODO: is Some(value.at_host()) correct?
        Self {
            local: value.local(),
            host: Some(value.at_host()),
        }
    }
}

/// Structure that describes a path within the conext of the Fermah app. It provides a path inside of the Fermah app directory.
/// This is done for 1. keeping clean the host machine from the app storing data everywhere, 2. easiness of sharing the data with
/// containers that are created by the app, especially, when the app is run from a container itself.
#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PathBufMirror {
    root: PathBuf,
    root_at_host: Option<PathBuf>,
    postfix: PathBuf,
}

// Note:    It is quite arbitrary and depends on the usage what do we want
//          to make async: constructors or getters. We could avoid using async even, by using std::fs methods.
//          I let constructors to be async and sync, and lets just remove something when we see the need.
//          Sync constructor is mostly used for ser/de.

impl PathBufMirror {
    pub fn new(postfix: PathBuf, root: PathBuf, root_at_host: Option<PathBuf>) -> Self {
        Self {
            root,
            root_at_host,
            // app_home: std::env::var("ASSETS_LOCATION").unwrap().into(),
            // app_home_at_host: std::env::var("ASSETS_LOCATION_HOST").ok().map(|s| s.into()),
            postfix,
        }
    }

    pub async fn new_at_assets(postfix: PathBuf) -> Result<Self, Error> {
        Ok(Self::new(
            postfix,
            app_home_dir().await?.join(ASSETS_DIRECTORY),
            app_home_dir_at_host()
                .await?
                .map(|p| p.join(ASSETS_DIRECTORY)),
        ))
    }

    pub fn new_at_assets_sync(postfix: PathBuf) -> Result<Self, Error> {
        Ok(Self::new(
            postfix,
            app_home_dir_sync()?.join(ASSETS_DIRECTORY),
            app_home_dir_at_host_sync()?.map(|p| p.join(ASSETS_DIRECTORY)),
        ))
    }

    pub async fn from_str(s: &str) -> Result<Self, Error> {
        let postfix = PathBuf::from_str(s).expect("Infalliable");
        Self::new_at_assets(postfix).await
    }

    pub fn from_str_sync(s: &str) -> Result<Self, Error> {
        let postfix = PathBuf::from_str(s).expect("Infalliable");
        Self::new_at_assets_sync(postfix)
    }

    pub fn local(&self) -> PathBuf {
        self.root.join(&self.postfix)
    }

    pub fn at_host(&self) -> PathBuf {
        self.root_at_host
            .as_ref()
            .unwrap_or(&self.root)
            .join(&self.postfix)
    }

    pub fn join<P>(&self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let postfix = self.postfix.join(path);
        Self {
            root_at_host: self.root_at_host.clone(),
            root: self.root.clone(),
            postfix,
        }
    }

    pub fn push<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        self.postfix.push(path);
    }

    pub fn exists(&self) -> bool {
        self.local().exists()
    }
}

pub mod path_buf_mirror_serde {

    use serde::{Deserialize, Deserializer, Serializer};

    use super::PathBufMirror;

    pub fn serialize<S>(x: &PathBufMirror, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(
            x.postfix
                .to_str()
                .expect("If it is not serializable it is okay, as this is only a dev feature"),
        )
    }

    pub fn deserialize<'de, D>(d: D) -> Result<PathBufMirror, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(d)?;
        // Taken that PathBufMirror initialize method is async,
        PathBufMirror::from_str_sync(&buf).map_err(serde::de::Error::custom)
    }
}

// Notice:  First of all, PathBufMountable wants to be serde only for usage of `LocalResource` in `Executable`, but it is really wrong, as it
//          implies that the same file which we can find in this system, we should be able to find in another system.
pub mod path_buf_mountable_serde {

    use std::{path::PathBuf, str::FromStr};

    use serde::{Deserialize, Deserializer, Serializer};

    use super::PathBufMountable;

    pub fn serialize<S>(x: &PathBufMountable, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(
            x.local()
                .to_str()
                .expect("If it is not serializable it is okay, as this is only a dev feature"),
        )
    }

    pub fn deserialize<'de, D>(d: D) -> Result<PathBufMountable, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(d)?;
        // Taken that PathBufMountable initialize method is async,
        Ok(PathBufMountable {
            local: PathBuf::from_str(&buf).map_err(serde::de::Error::custom)?,
            host: None,
        })
    }
}
