pub mod error;
pub mod keystore;
pub mod profile;

use std::{collections::HashMap, fmt::Display, path::Path};

use fermah_common::types::network::Network;
use serde::{de::DeserializeOwned, Serialize};
use tokio::fs;
use tracing::info;

use crate::{
    error::Error,
    profile::{key::ProfileKey, Profile, ProfileType},
};

/// A collection of profiles indexed by a [ProfileKey].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// use serde::{Deserialize, Serialize};
/// use fermah_config::profile::ProfileType;
/// use fermah_config::Profiles;
/// use fermah_config::profile::key::ProfileKey;
/// use fermah_config::error::Error;
/// use fermah_common::types::network::Network;
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
/// let profiles: Profiles<TestConfig> =
///     Profiles::from_dir(&PathBuf::from("config"), &Network::Local, &ProfileType::Proof).await?;
///     
///     // Search for a profile by name, type, and network
///     let profile = profiles.index.get(&ProfileKey {
///         name: "test".to_string(),
///         network: Network::Local,
///     }).unwrap();
///     
///     Ok(())
/// }
pub struct Profiles<T> {
    pub index: HashMap<ProfileKey, Profile<T>>,
}

impl<T> Display for Profiles<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Name\t\tNetwork")?;
        for (key, _) in self.index.iter() {
            writeln!(f, "{}", key)?;
        }
        Ok(())
    }
}

impl<T: Serialize + DeserializeOwned + Clone> Profiles<T> {
    /// Load all profiles of a given type from a directory.
    pub async fn from_dir(
        base_dir: &Path,
        network: &Network,
        profile_type: &ProfileType,
    ) -> Result<Self, Error> {
        let dir = base_dir.join(format!("{}net", network));

        info!("loading profiles from {}", dir.display());

        let mut index: HashMap<ProfileKey, Profile<T>> = HashMap::new();
        let mut entries = fs::read_dir(dir).await?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let file_name = path
                .file_name()
                .ok_or(Error::InvalidPath(path.clone()))?
                .to_str()
                .ok_or(Error::NonUtf8Path(path.clone()))?;
            if !file_name.starts_with(&profile_type.to_string()) {
                continue;
            }

            let profile = Profile::<T>::from_path(&path).await?;
            info!("found profile {}", file_name);
            index.insert(profile.build_key(), profile);
        }

        Ok(Self { index })
    }
}

/// A wrapper around a JSON file that can be deserialized into a type `T`.
pub struct ConfigFile<T>(pub T, std::marker::PhantomData<()>);

impl<T: DeserializeOwned> ConfigFile<T> {
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<T, Error> {
        let file_contents = tokio::fs::read(path.as_ref()).await?;
        let config: T = serde_json::from_slice(&file_contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;
    use fermah_common::types::network::Network;
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::profile::{
        command::{MergableArgs, ProfileCommands},
        FromProfile,
    };

    /// Fermah Configuration CLI.
    #[derive(Parser, Debug)]
    #[command(version, about, long_about = None)]
    pub struct Cli {
        /// Commands
        #[command(subcommand)]
        pub command: ProfileCommands<TestConfigArgs>,
    }

    #[derive(Serialize, Deserialize, Parser, Debug)]
    pub struct TestConfigArgs {
        /// Data
        #[arg(short = 'd', long)]
        pub data: String,
    }

    impl MergableArgs for TestConfigArgs {
        type Error = ();
        type MergeType = TestConfig;

        async fn merge(&self, other: Self::MergeType) -> Result<Self::MergeType, Self::Error> {
            Ok(TestConfig {
                data: self.data.clone(),
                ..other
            })
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct TestConfig {
        data: String,
        advanced_data: String,
    }

    /// Run tests in sequence to avoid fs conflicts.
    #[tokio::test]
    async fn sequential() -> Result<(), Error> {
        new_profiles().await?;
        commands().await?;
        Ok(())
    }

    /// Test that we can create a profile, save it, and then load it back.
    /// We also test that we can load all profiles of a given type from a directory.
    async fn new_profiles() -> Result<(), Error> {
        let dir = PathBuf::from("config");

        let profile1 = Profile::new(
            dir.clone(),
            "test".to_string(),
            "test description".to_string(),
            Network::Local,
            ProfileType::Proof,
            TestConfig {
                data: "data".to_string(),
                advanced_data: "data".to_string(),
            },
        );

        profile1.save().await?;

        let profile_key = ProfileKey {
            name: "test".to_string(),
            network: Network::Local,
        };

        let profile2 =
            Profile::<TestConfig>::from_props(&dir, ProfileType::Proof, &profile_key).await?;
        assert_eq!(profile2.name, "test");

        let profiles =
            Profiles::<TestConfig>::from_dir(&dir, &Network::Local, &ProfileType::Proof).await?;

        assert_eq!(profiles.index.len(), 3);

        assert_eq!(profiles.index.get(&profile_key).unwrap().name, "test");

        Ok(())
    }

    /// Test that we can list, get, and set profiles from the subcommand.
    async fn commands() -> Result<(), Error> {
        let list = Cli::parse_from(vec!["", "list", "-k", "local"]).command;
        let get = Cli::parse_from(vec!["", "get", "-n", "cli", "-k", "local"]).command;
        let set = Cli::parse_from(vec!["", "set", "-n", "cli", "-k", "local", "-d", "new"]).command;

        let set_tmpl = Cli::parse_from(vec![
            "", "set", "-n", "tmpl", "-k", "local", "-d", "new", "-t", "default",
        ])
        .command;
        let del = Cli::parse_from(vec!["", "del", "-n", "tmpl", "-k", "local"]).command;

        let dir = PathBuf::from("config");

        list.run(ProfileType::Proof, &dir).await?;
        get.run(ProfileType::Proof, &dir).await?;
        set.run(ProfileType::Proof, &dir).await?;

        set_tmpl.run(ProfileType::Proof, &dir).await?;

        let key = ProfileKey {
            name: "tmpl".to_string(),
            network: Network::Local,
        };

        let config = TestConfig::from_profile(&dir, ProfileType::Proof, &key).await?;

        assert_eq!(config.advanced_data, "default");

        del.run(ProfileType::Proof, &dir).await?;

        Ok(())
    }
}
