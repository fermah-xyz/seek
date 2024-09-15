use std::{future::Future, path::Path};

use serde::{de::DeserializeOwned, Serialize};
use tokio::fs;
use tracing::{debug, error, info};

use crate::fs::{ensure_dir, error::Error};

/// A trait for deserializing from a JSON file, any type that implements Deserialize.
pub trait Json: Sized {
    fn from_json_path<P: AsRef<Path> + Send>(
        path: P,
    ) -> impl Future<Output = Result<Self, Error>> + Send
    where
        Self: DeserializeOwned,
    {
        async {
            info!("reading {}", path.as_ref().display());

            if !path.as_ref().exists() {
                error!("file not found: {}", path.as_ref().display())
            }
            let file_contents = fs::read(path).await?;

            debug!("{}", String::from_utf8_lossy(&file_contents));

            let result = serde_json::from_slice(&file_contents)?;

            Ok(result)
        }
    }

    fn to_json_path<P: AsRef<Path> + Send>(
        &self,
        path: P,
    ) -> impl Future<Output = Result<(), Error>>
    where
        Self: Serialize,
    {
        async {
            let json = serde_json::to_string_pretty(self)?;
            let parent = path.as_ref().parent().unwrap();
            ensure_dir(&parent, None).await?;
            fs::write(path, json).await?;
            Ok(())
        }
    }
}

impl<T> Json for T where T: Serialize + DeserializeOwned {}
