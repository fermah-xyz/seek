use std::path::PathBuf;

use rand::distributions::{Alphanumeric, DistString};
use rand_core::RngCore;
use tokio::fs::File;
use uuid::Uuid;

use crate::fs::{dir_from_env, ensure_dir};

pub enum RandomPathType {
    File,
    Dir,
}

pub async fn random_path(
    path_type: RandomPathType,
    len: usize,
    mut rng: impl RngCore,
) -> anyhow::Result<(PathBuf, Option<File>), std::io::Error> {
    let mut base_path = dir_from_env(
        "RANDOM_DATA_DIRECTORY",
        Uuid::new_v4().to_string(),
        std::env::temp_dir(),
    )
    .await?;

    ensure_dir(&base_path, None).await?;

    let random_name: String = Alphanumeric.sample_string(&mut rng, len);
    base_path.push(random_name);

    match path_type {
        RandomPathType::File => {
            let file = File::create(&base_path).await?;
            Ok((base_path, Some(file)))
        }
        RandomPathType::Dir => {
            ensure_dir(&base_path, None).await?;
            Ok((base_path, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand_core::SeedableRng;

    use super::*;

    #[tokio::test]
    async fn test_random_file() {
        let (path, file) = random_path(RandomPathType::File, 16, StdRng::seed_from_u64(0))
            .await
            .unwrap();

        assert!(path.exists());
        assert!(file.is_some());
    }
}
