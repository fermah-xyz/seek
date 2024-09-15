pub mod avs;
pub mod el;
pub mod erc20;
pub mod fermah;
pub mod strategy;

use std::sync::Arc;

use anyhow::{Context, Result};
use avs::AVSContracts;
use el::ELContracts;
use ethers::{
    middleware::MiddlewareBuilder,
    prelude::{Http, Provider, Signer},
};
use fermah_common::crypto::signer::ecdsa::EcdsaSigner;
use url::Url;

use self::fermah::FermahContracts;
use crate::{config::Config, SignerMiddlewareContract};

#[derive(Clone)]
pub struct Contracts {
    pub avs_contracts: AVSContracts,
    pub fermah_contracts: FermahContracts,
    pub el_contracts: ELContracts,
    // Uh, oh, this is so dirty to have provider here and in the contracts
    pub provider: Arc<SignerMiddlewareContract>,
}

impl Contracts {
    pub async fn from_config(config: &Config, rpc: &Url, signer: EcdsaSigner) -> Result<Self> {
        let client = Arc::new(
            Provider::<Http>::try_from(&rpc.to_string()).context("failed to create provider")?,
        );
        let signer = signer.with_chain_id(config.chain_id);
        let provider = Arc::new(client.with_signer::<EcdsaSigner>(signer));

        Ok(Self {
            avs_contracts: AVSContracts::new(config, provider.clone()),
            fermah_contracts: FermahContracts::new(config, provider.clone()),
            el_contracts: ELContracts::new(config, provider.clone()),
            provider,
        })
    }
}
