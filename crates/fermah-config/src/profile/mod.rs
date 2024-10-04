use std::{
    fmt::Display,
    future::Future,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use fermah_common::{fs::json::Json, types::network::Network};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use strum::Display;
use tokio::fs;
use tracing::info;

pub mod command;
pub mod key;

use crate::{error::Error, profile::key::ProfileKey};

pub const CONFIG_DIR: &str = "config";
pub const NONCE_FILE: &str = "nonce";

#[derive(
    Serialize, Deserialize, Display, ValueEnum, Default, Debug, Clone, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ProfileType {
    #[default]
    Proof,
    Operator,
    Registration,
    Matchmaker,
    Avs,
    Telemetry,
}

/// A profile holds a single instance of a configuration for a given cfg type and network.
/// It is stored in a file named after the profile type, name, and network.
///
/// # Example
///
/// ```
///  use std::path::PathBuf;
///
///  use serde::{Deserialize, Serialize};
///  use fermah_config::profile::{Profile, ProfileType};
///  use fermah_config::error::Error;
///  use fermah_config::profile::key::ProfileKey;
///  use fermah_common::types::network::Network;
///
///  #[derive(Serialize, Deserialize, Debug, Clone)]
///  #[serde(rename_all = "camelCase")]
///  struct TestConfig {
///     data: String,
///     advanced_data: String,
///  }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
/// let config_dir = PathBuf::from("config");
///
/// let profile = Profile::<TestConfig>::new(
///         config_dir.clone(),
///         "test".to_string(),
///         "test profile".to_string(),
///         Network::Local,
///         ProfileType::Operator,
///         TestConfig {
///             data: "data".to_string(),
///             advanced_data: "advanced_data".to_string(),
///         });
///
///     let profile_key = ProfileKey {
///         name: "test".to_string(),
///         network: Network::Local,
///     };
///
///     let profile_from_path = Profile::<TestConfig>::from_props(&config_dir, ProfileType::Proof, &profile_key).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Serialize, Deserialize)]
pub struct Profile<T> {
    #[serde(skip)]
    pub path: PathBuf,

    pub name: String,
    pub description: String,
    pub network: Network,

    #[serde(rename = "type")]
    pub profile_type: ProfileType,

    pub config: T,
}

impl<T> Display for Profile<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}:{})",
            self.path.display(),
            self.name,
            self.network
        )
    }
}

impl<T: Serialize + DeserializeOwned> Profile<T> {
    pub fn new(
        dir: PathBuf,
        name: String,
        description: String,
        network: Network,
        profile_type: ProfileType,
        config: T,
    ) -> Self {
        let path = Profile::<T>::build_path(&dir, &network, &profile_type, &name);

        Self {
            path,
            name: name.clone(),
            description,
            network: network.clone(),
            profile_type,
            config,
        }
    }

    /// Load a profile from a file path.
    /// Checks for key mismatch between the file path and the loaded profile.
    pub async fn from_path(path: &Path) -> Result<Self, Error> {
        let mut profile = Profile::from_json_path(path).await?;
        profile.path = path.to_path_buf();
        Ok(profile)
    }

    pub async fn from_props(
        dir: &Path,
        profile_type: ProfileType,
        ProfileKey { network, name }: &ProfileKey,
    ) -> Result<Self, Error> {
        let path = Profile::<T>::build_path(dir, network, &profile_type, name);
        Self::from_path(&path).await
    }

    pub fn build_key(&self) -> ProfileKey {
        ProfileKey {
            name: self.name.clone(),
            network: self.network.clone(),
        }
    }

    pub fn build_path(
        dir: &Path,
        network: &Network,
        profile_type: &ProfileType,
        name: &String,
    ) -> PathBuf {
        PathBuf::from(dir)
            .join(format!("{}net", network))
            .join(format!("{}.{}.json", profile_type, name))
    }

    pub async fn save(&self) -> Result<(), Error> {
        self.to_json_path(&self.path).await?;
        info!("saved profile: {}", self.path.display());
        Ok(())
    }

    pub async fn delete(&self) -> Result<(), Error> {
        fs::remove_file(&self.path).await?;
        info!("deleted profile: {}", self.path.display());
        Ok(())
    }
}

/// A trait for deserializing from base dir and profile props, any type that implements Deserialize.
pub trait FromProfile: Sized {
    fn from_profile(
        cfg_dir: &Path,
        profile_type: ProfileType,
        profile_key: &ProfileKey,
    ) -> impl Future<Output = Result<Self, Error>> + Send
    where
        Self: Serialize + DeserializeOwned,
    {
        {
            async move {
                let profile =
                    Profile::<Self>::from_props(cfg_dir, profile_type, profile_key).await?;
                Ok(profile.config)
            }
        }
    }
}

impl<T> FromProfile for T where T: DeserializeOwned {}
