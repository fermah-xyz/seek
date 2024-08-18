use std::path::PathBuf;

use fermah_avs::SignerMiddlewareContract;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("fs error: {0}")]
    Fs(#[from] fermah_common::fs::error::Error),
    #[error("cfg error: {0}")]
    Cfg(#[from] fermah_config::error::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse url: {0}")]
    Url(#[from] url::ParseError),
    #[error("rpc client error: {0}")]
    RpcClient(#[from] fermah_rpc::rpc_client::RpcClientError),
    #[error("contract error: {0}")]
    Contract(#[from] ethers_contract::ContractError<SignerMiddlewareContract>),
    #[error("keystore error: {0}")]
    Keystore(#[from] fermah_config::keystore::error::Error),
    #[error("keystore file error: {0}")]
    KeystoreFile(#[from] fermah_common::crypto::keystore::KeystoreFileError),
    #[error("file download error: {0}")]
    FileDownload(#[from] fermah_common::http::file_download::FileDownloadError),
    #[error("file already exists: {0}")]
    FileExists(PathBuf),
    #[error("invalid file url")]
    InvalidFileUrl,

    #[error("error: {0}")]
    Other(#[from] anyhow::Error),
}
