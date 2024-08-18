use std::{fmt::Display, path::Path};

use clap::{Parser, ValueEnum};
use fermah_common::types::network::Network;

use crate::error::Error;

/// A profile key is a unique identifier for a profile.
/// It consists of a network and profile name.
/// It's used to index profiles in a collection.
#[derive(Parser, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileKey {
    /// Configuration network
    #[arg(short = 'k', long)]
    pub network: Network,

    /// Configuration profile name
    #[arg(short = 'n', long = "profile", default_value = "default")]
    pub name: String,
}

impl ProfileKey {
    pub fn to_whitelister_profile(&self, avs_profile_name: String) -> Self {
        Self {
            network: self.network.clone(),
            name: format!("{avs_profile_name}.whitelister"),
        }
    }

    pub fn to_minter_profile(&self) -> Self {
        Self {
            network: self.network.clone(),
            name: format!("{}.minter", self.name),
        }
    }
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        let file_name = path
            .file_name()
            .ok_or(Error::InvalidPath(path.to_path_buf()))?
            .to_str()
            .ok_or(Error::NonUtf8Path(path.to_path_buf()))?
            .split('.')
            .collect::<Vec<&str>>();

        let network = path
            .parent()
            .ok_or(Error::InvalidPath(path.to_path_buf()))?
            .file_name()
            .ok_or(Error::InvalidPath(path.to_path_buf()))?
            .to_str()
            .ok_or(Error::NonUtf8Path(path.to_path_buf()))?
            .to_string()
            .replace("net", "");

        if file_name.len() != 3 {
            return Err(Error::InvalidPath(path.to_path_buf()));
        }

        Ok(Self {
            name: file_name[1].to_string(),
            network: Network::from_str(network.as_str(), true).unwrap(),
        })
    }
}

impl Display for ProfileKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t\t{}", self.name, self.network)
    }
}
