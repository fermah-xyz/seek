use std::path::Path;

use clap::Subcommand;
use fermah_common::types::network::Network;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info};

use crate::{
    error::Error,
    profile::{Profile, ProfileType},
    ProfileKey,
    Profiles,
};

/// Profile CLI configuration subcommands
#[derive(Subcommand, Debug)]
pub enum ProfileCommands<A: MergableArgs> {
    /// List profiles
    #[command(alias = "l")]
    List {
        /// Network
        #[arg(short = 'k', long)]
        network: Network,
    },
    /// Get and print profile
    #[command(alias = "g")]
    Get {
        #[command(flatten)]
        profile: ProfileKey,
    },
    /// Set profile's config
    #[command(alias = "s")]
    Set {
        /// Profile template
        #[arg(short, long, default_value = "default")]
        template: String,

        /// Composite profile key
        #[command(flatten)]
        profile: ProfileKey,

        /// Inner configuration args
        #[command(flatten)]
        config: A,
    },
    /// Delete profile
    #[command(alias = "d")]
    Del {
        #[command(flatten)]
        profile: ProfileKey,
    },
}

/// A trait for merging configuration arguments
pub trait MergableArgs: clap::Args {
    type Error;
    type MergeType: Serialize + DeserializeOwned + Clone;

    fn merge(
        &self,
        other: Self::MergeType,
    ) -> impl std::future::Future<Output = Result<Self::MergeType, Self::Error>> + Send;
}

impl<A: MergableArgs> ProfileCommands<A> {
    pub async fn run(&self, profile_type: ProfileType, config_dir: &Path) -> Result<(), Error> {
        match &self {
            ProfileCommands::List { network } => {
                info!("listing profiles for {}:{}", profile_type, network);

                let profiles: Profiles<A::MergeType> =
                    Profiles::from_dir(config_dir, network, &profile_type).await?;

                println!("\n{}", profiles);
            }
            ProfileCommands::Get { profile } => {
                info!("getting {:?}", profile);

                let profiles: Profiles<A::MergeType> =
                    Profiles::from_dir(config_dir, &profile.network, &profile_type).await?;

                let p = profiles.index.get(profile).ok_or(Error::ProfileNotFound {
                    profile: profile.clone(),
                    template: None,
                    dir: config_dir.to_path_buf(),
                })?;

                println!("\n{}", serde_json::to_string_pretty(&p.config)?);
            }
            ProfileCommands::Del { profile } => {
                info!("deleting {:?}", profile);

                Profile::<A::MergeType>::from_props(config_dir, profile_type, profile)
                    .await?
                    .delete()
                    .await?;
            }
            ProfileCommands::Set {
                template,
                profile,
                config,
            } => {
                info!("setting {:?}", profile);

                let mut profiles: Profiles<A::MergeType> =
                    Profiles::from_dir(config_dir, &profile.network, &profile_type).await?;

                let p = match profiles.index.get_mut(profile) {
                    Some(p) => p,
                    None => {
                        let tmpl_key = &ProfileKey {
                            name: template.clone(),
                            ..profile.clone()
                        };

                        info!(
                            "profile not found, creating new profile based on template: {:?}",
                            tmpl_key
                        );

                        let p = profiles
                            .index
                            .get_mut(tmpl_key)
                            .ok_or(Error::ProfileNotFound {
                                profile: profile.clone(),
                                template: Some(template.clone()),
                                dir: config_dir.to_path_buf(),
                            })?;

                        // Need to update the profile name to match the new profile
                        // As well as the path it will be saved to
                        p.name.clone_from(&profile.name);
                        p.path = Profile::<A::MergeType>::build_path(
                            config_dir,
                            &profile.network,
                            &profile_type,
                            &profile.name,
                        );

                        p
                    }
                };

                p.config = config.merge(p.config.clone()).await.map_err(|_| {
                    Error::Merge {
                        profile: profile.clone(),
                    }
                })?;
                p.save().await?;

                info!("profile saved to: {}", p.path.display());
                debug!("\n{}", serde_json::to_string_pretty(&p.config)?);
            }
        }

        Ok(())
    }
}
