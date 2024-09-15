use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fermah_common::{
    fs::json::Json,
    manifest::{ElManifestConfig, FermahManifestConfig},
};
use fermah_config::profile::{key::ProfileKey, Profile, ProfileType};
use tracing::error;

use crate::config::Config;

pub async fn merge_manifests(config_dir: &Path, profile_key: &ProfileKey) -> Result<()> {
    let el_json = PathBuf::from(format!(
        "contracts/script/output/el_deployment.{}.json",
        profile_key.network
    ));

    let fermah_json = PathBuf::from(format!(
        "contracts/script/output/middleware_deployment.{}.json",
        profile_key.network
    ));

    let el_config = ElManifestConfig::from_json_path(&el_json)
        .await
        .with_context(|| format!("failed to parse EL manifest: {el_json:?}"))?;
    let fermah_config = FermahManifestConfig::from_json_path(&fermah_json)
        .await
        .with_context(|| format!("failed to parse Fermah manifest: {fermah_json:?}"))?;

    if el_config.chain_info.chain_id != fermah_config.chain_info.chain_id {
        error!(el=?el_config.chain_info.chain_id, fermah=?fermah_config.chain_info.chain_id, "EL and Fermah smart contracts should be deployed to the same blockchain");
        anyhow::bail!("chain ID differs");
    }

    let mut avs_profile =
        Profile::<Config>::from_props(config_dir, ProfileType::Avs, profile_key).await?;
    avs_profile.config.merge(&el_config, &fermah_config);
    avs_profile
        .save()
        .await
        .with_context(|| format!("failed to update profile: {config_dir:?}"))
}
