use tokio::fs::File;
use tracing::debug;
use uuid::Uuid;

use super::mountable::PathBufMirror;
use crate::fs::{ensure_dir, Error};

pub enum RandomPathType {
    File,
    Dir,
}

pub async fn random_path() -> Result<PathBufMirror, Error> {
    let mut base_path = PathBufMirror::from_str("random").await?;

    ensure_dir(&base_path.local(), None).await?;

    base_path.push(Uuid::new_v4().to_string());
    Ok(base_path)
}

pub async fn new_at_random_path(
    path_type: RandomPathType,
) -> Result<(PathBufMirror, Option<File>), Error> {
    let mut base_path = PathBufMirror::from_str("random").await?;

    ensure_dir(&base_path.local(), None).await?;

    base_path.push(Uuid::new_v4().to_string());

    match path_type {
        RandomPathType::File => {
            let file = File::create(&base_path.local()).await?;
            debug!(?base_path, local=?base_path.local(), "Created a file");
            Ok((base_path, Some(file)))
        }
        RandomPathType::Dir => {
            ensure_dir(&base_path.local(), None).await?;
            Ok((base_path, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_random_file() {
        let (path, file) = new_at_random_path(RandomPathType::File).await.unwrap();

        assert!(path.exists());
        assert!(file.is_some());

        assert!(tokio::fs::remove_file(&path.local()).await.is_ok());
        assert!(!path.exists());
    }
}
