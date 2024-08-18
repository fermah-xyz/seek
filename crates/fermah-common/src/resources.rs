use std::{env::VarError, fs::File, io::Write, path::PathBuf, str::FromStr};

use futures_util::stream::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error;
use tracing::{debug, error, warn};
use url::Url;

use crate::{
    fs::ensure_dir,
    hash::{
        blake3::{Blake3Hash, Blake3Hasher},
        Hasher,
    },
    serialization::encoding::hex_encoded,
};

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalResource {
    /// URL to a HTTP endpoint where the image can be downloaded.
    pub path: PathBuf,
    /// [`blake3`] hash of the program image.
    #[serde(with = "hex_encoded")]
    pub hash: Blake3Hash,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteResource {
    /// URL to a HTTP endpoint where the image can be downloaded.
    pub url: Url,
    /// [`blake3`] hash of the program image.
    #[serde(with = "hex_encoded")]
    pub hash: Blake3Hash,
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Remote resource not found: {0}")]
    NotFound(Url),

    #[error("Hash mismatch for url {url}: {expected} != {found}")]
    HashMismatch {
        url: Url,
        expected: Blake3Hash,
        found: Blake3Hash,
    },
}

impl RemoteResource {
    /// Download the program image to a local file
    /// and check if its hash matches the computed hash.
    pub async fn download(&self, path: Option<PathBuf>) -> Result<PathBuf, DownloadError> {
        // todo: probably treat differently dirs and individual files?
        let location = path.unwrap_or(Self::root().await?.join(format!("{}", self.hash)));

        debug!(
            "Going to store new file from {} to {:?}",
            self.url, location
        );
        if location.exists() {
            error!(?location, "File exists");
            return Ok(location);
        }

        // Create temporary file
        let tmp_file = NamedTempFile::new()?;
        let tmp_file_name = tmp_file.path().to_owned();
        let mut file = File::create(&tmp_file_name)?;

        // Download file.
        let response = reqwest::get(self.url.as_ref()).await?;

        let response = response.error_for_status();
        match response {
            Ok(response) => {
                let mut stream = response.bytes_stream();

                let mut hasher = Blake3Hasher::new();
                // Write image to a temporary file and compute its hash.
                while let Some(Ok(item)) = stream.next().await {
                    hasher.update(&item);
                    file.write_all(&item)?;
                }

                // Check hash and persist the file.
                let hash = hasher.finalize();

                if hash != self.hash {
                    // Ignore if remove_file; it will be removed by the OS anyway.
                    let _ = std::fs::remove_file(&tmp_file_name);
                    error!(expected=?self.hash, got=?hash, "Invalid hash");
                    Err(DownloadError::HashMismatch {
                        url: self.url.clone(),
                        expected: self.hash,
                        found: hash,
                    })?
                }

                std::fs::rename(&tmp_file_name, &location)?;
                Ok(location)
            }
            Err(e) => {
                error!(err=?e, "failed to download");
                let status_code = e.status();
                if let Some(status_code) = status_code {
                    if matches!(status_code, StatusCode::NOT_FOUND) {
                        Err(DownloadError::NotFound(self.url.clone()))?
                    }
                }
                Err(e)?
            }
        }
    }

    pub async fn root() -> Result<PathBuf, std::io::Error> {
        let root = match std::env::var("DOWNLOAD_DIRECTORY") {
            Ok(directory) => {
                PathBuf::from_str(&directory).expect("PathBuf::from_str is unfalliable")
            }
            Err(e) => {
                let default_root = Self::default_root();
                if let VarError::NotUnicode(contents) = e {
                    warn!(
                        ?contents,
                        ?default_root,
                        "Value of env var DOWNLOAD_DIRECTORY is not unicode. Using default"
                    );
                }
                default_root
            }
        };

        ensure_dir(&root, None).await?;
        Ok(root)
    }

    pub fn default_root() -> PathBuf {
        let mut target = std::env::temp_dir();
        target.push("download_storage");
        target
    }

    pub async fn into_local(self) -> Result<LocalResource, DownloadError> {
        let path = self.download(None).await?;
        Ok(LocalResource {
            hash: self.hash,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let rrs = vec![RemoteResource {
            url: "http://localhost:8082/dummy_prover_latest.tar.gz"
                .parse()
                .unwrap(),
            hash: Blake3Hash(blake3::Hash::from_bytes([
                50, 235, 26, 34, 170, 83, 73, 153, 59, 164, 55, 11, 174, 204, 153, 4, 87, 3, 75,
                158, 8, 187, 32, 156, 174, 44, 132, 64, 14, 121, 100, 140,
            ])),
        }];

        let s = serde_json::to_string_pretty(&rrs).unwrap();

        println!("{}", s);

        let rs: Vec<RemoteResource> = serde_json::from_str(&s).unwrap();
        assert_eq!(rrs, rs);
        println!("{:?}", rs);

        let x = bincode::serialize(&rs).unwrap();

        let x = bincode::deserialize::<Vec<RemoteResource>>(&x).unwrap();

        assert_eq!(x, rs)
    }
}
