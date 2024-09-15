use std::path::Path;

use fermah_common::fs::{app_home_dir, copy_dir};
use fermah_config::profile::CONFIG_DIR;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home_config_dir = app_home_dir().await?.join(CONFIG_DIR);
    let templates_config_dir = Path::new("config");

    let dirs = vec!["localnet", "devnet", "testnet", "mainnet"];

    for d in dirs {
        copy_dir(&templates_config_dir.join(d), &home_config_dir.join(d)).await?;
    }

    Ok(())
}
