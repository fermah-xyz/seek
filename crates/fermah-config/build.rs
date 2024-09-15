use std::path::Path;

use fermah_common::fs::{app_home_dir, copy_dir};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let home_config_dir = app_home_dir().await?.join("keys");
    let keys_dir = Path::new("keys");

    copy_dir(&keys_dir, &home_config_dir).await?;

    Ok(())
}
