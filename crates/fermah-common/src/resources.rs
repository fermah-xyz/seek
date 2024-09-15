use std::io::Write;

use futures_util::stream::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error};
use url::Url;

use crate::{
    fs::{
        ensure_dir,
        error::Error as FsError,
        mountable::{path_buf_mirror_serde, PathBufMirror},
    },
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
    #[serde(with = "path_buf_mirror_serde")]
    pub path: PathBufMirror,
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

    #[error("Fermah-fs error: {0}")]
    FsError(#[from] FsError),

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
    pub async fn download(
        &self,
        path: Option<PathBufMirror>,
    ) -> Result<PathBufMirror, DownloadError> {
        // todo: probably treat differently dirs and individual files?
        let location = path.unwrap_or(Self::root().await?.join(format!("{}", self.hash)));

        debug!(
            "Going to store new file from {} to {:?}",
            self.url,
            location.local()
        );
        if location.exists() {
            debug!(?location, "File exists");
            return Ok(location);
        }

        // Create temporary file
        #[cfg(not(feature = "dockerized"))]
        // If we don't work with dockerization, we can create temop files anywhere, including in tmp, which belong to existing FS (hopefully)
        let mut file = tempfile::NamedTempFile::new()?;
        #[cfg(feature = "dockerized")]
        // Notice:  We need to be very careful, so that this file won't be used, while we are figuring out if this file is legal (ex.: hash is correct).
        //          This is why we create a temp file inside of the mounted FS thanks to `PathBufMirror` from `random_path`.
        let (mut file, tmp_file_location) = Self::temp_file().await?;

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
                    #[cfg(not(feature = "dockerized"))]
                    // Generally, we can ignore the `file`, as it will be removed automatically when if gets out of scope. But, it could be
                    // more readable to do the explicit `drop` here.
                    drop(file);
                    #[cfg(feature = "dockerized")]
                    // With `dockerized` setup we created the temp file ourselves in the mounted FS, thus we need to take care about it ourselves too.
                    let _ = std::fs::remove_file(tmp_file_location.local());

                    error!(expected=?self.hash, got=?hash, "Invalid hash");
                    Err(DownloadError::HashMismatch {
                        url: self.url.clone(),
                        expected: self.hash,
                        found: hash,
                    })?
                } else {
                    #[cfg(not(feature = "dockerized"))]
                    std::fs::rename(file.path(), location.local())?;
                    #[cfg(feature = "dockerized")]
                    std::fs::rename(tmp_file_location.local(), location.local())?;

                    Ok(location)
                }
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

    pub async fn root() -> Result<PathBufMirror, DownloadError> {
        let download_root = PathBufMirror::from_str("downloads").await?;

        ensure_dir(&download_root.local(), None).await?;
        Ok(download_root)
    }

    #[cfg(feature = "dockerized")]
    pub async fn temp_file() -> Result<(std::fs::File, PathBufMirror), DownloadError> {
        use crate::fs::rand::random_path;

        let file_location = random_path().await?;

        let file = std::fs::File::create(file_location.local())?;

        Ok((file, file_location))
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
