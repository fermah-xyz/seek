use std::collections::HashMap;

use ethers::types::Address;
use fermah_common::manifest::{ElManifestConfig, FermahManifestConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub chain_id: u64,
    pub avs_contract: AvsContract,
    pub fermah_contract: FermahContract,
    pub el_contract: ElContract,
}

impl Config {
    pub fn merge(&mut self, el_config: &ElManifestConfig, fermah_config: &FermahManifestConfig) {
        self.avs_contract.operator_state_retriever =
            fermah_config.addresses.operator_state_retriever;
        self.avs_contract.registry_coordinator = fermah_config.addresses.registry_coordinator;

        self.fermah_contract.dispute_manager = fermah_config.addresses.dispute_manager;
        self.fermah_contract.service_manager = fermah_config.addresses.service_manager;
        self.fermah_contract.vault = fermah_config.addresses.vault;
        self.fermah_contract.vault_token = fermah_config.addresses.vault_token;
        self.fermah_contract.whitelist = fermah_config.addresses.whitelist;

        self.el_contract.avs_directory = el_config.addresses.avs_directory;
        self.el_contract.delegation_manager = el_config.addresses.delegation_manager;
        self.el_contract.strategy_manager = el_config.addresses.strategy_manager;
        self.el_contract.rewards_coordinator = el_config.addresses.rewards_coordinator;

        self.el_contract
            .strategies
            .clone_from(&el_config.addresses.strategies);

        self.chain_id = el_config.chain_info.chain_id;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvsContract {
    pub operator_state_retriever: Address,
    pub registry_coordinator: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FermahContract {
    pub dispute_manager: Address,
    pub service_manager: Address,
    pub vault: Address,
    pub vault_token: Address,
    pub whitelist: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElContract {
    pub avs_directory: Address,
    pub delegation_manager: Address,
    pub strategy_manager: Address,
    pub rewards_coordinator: Address,
    pub strategies: HashMap<String, Address>,
}
