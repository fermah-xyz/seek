pub mod avs;
pub mod config;
pub mod contract;
pub mod db;
pub mod error;
pub mod manifest;

use std::sync::Arc;

use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Provider},
};
use fermah_common::crypto::signer::ecdsa::EcdsaSigner;

pub type SignerMiddlewareContract = SignerMiddleware<Arc<Provider<Http>>, EcdsaSigner>;
