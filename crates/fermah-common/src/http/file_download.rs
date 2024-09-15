use std::path::Path;

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tracing::info;
use url::Url;

#[derive(thiserror::Error, Debug)]
pub enum FileDownloadError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("fs error: {0}")]
    Fs(#[from] crate::fs::error::Error),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("failed to get content length")]
    ContentLength,
}

pub struct FileDownload {
    pub url: Url,
}

impl FileDownload {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    pub async fn download_to_file<F>(
        &self,
        filepath: &Path,
        progress_callback: F,
    ) -> Result<(), FileDownloadError>
    where
        F: Fn(u64, u64) + Send + 'static,
    {
        info!("downloading file: {}", self.url);

        let res = reqwest::get(self.url.clone()).await?;
        match res.error_for_status() {
            Ok(res) => {
                let total_size = res
                    .content_length()
                    .ok_or(FileDownloadError::ContentLength)?;

                let mut downloaded_size: u64 = 0;

                let mut file = tokio::fs::File::create(filepath).await?;
                let mut stream = res.bytes_stream();

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    file.write_all(&chunk).await?;
                    downloaded_size += chunk.len() as u64;
                    progress_callback(downloaded_size, total_size);
                }

                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
