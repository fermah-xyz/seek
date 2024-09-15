pub mod avs;
pub mod config;
pub mod contract;
pub mod error;
pub mod manifest;

use std::sync::Arc;

use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Provider},
};
use fermah_common::crypto::signer::ecdsa::EcdsaSigner;

pub type SignerMiddlewareContract = SignerMiddleware<Arc<Provider<Http>>, EcdsaSigner>;

#[derive(Clone, PartialEq, Eq)]
pub enum ELOperatorStatus {
    Disabled,
    Active,
}

impl ELOperatorStatus {
    pub fn is_active(&self) -> bool {
        *self == Self::Active
    }
}

impl From<u8> for ELOperatorStatus {
    fn from(value: u8) -> Self {
        if value == 0 {
            ELOperatorStatus::Disabled
        } else {
            ELOperatorStatus::Active
        }
    }
}
