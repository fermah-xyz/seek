use std::collections::HashMap;

use ethers::types::Address;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElManifestConfig {
    pub addresses: ElAddresses,
    pub chain_info: ChainInfo,
    pub parameters: Parameters,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElAddresses {
    pub avs_directory: Address,
    pub avs_directory_implementation: Address,
    pub base_strategy_implementation: Address,
    pub delayed_withdrawal_router: Address,
    pub delayed_withdrawal_router_implementation: Address,
    pub delegation_manager: Address,
    pub delegation_manager_implementation: Address,
    pub eigen_layer_pauser_reg: Address,
    pub eigen_layer_proxy_admin: Address,
    pub eigen_pod_beacon: Address,
    pub eigen_pod_implementation: Address,
    pub eigen_pod_manager: Address,
    pub eigen_pod_manager_implementation: Address,
    pub empty_contract: Address,
    pub slasher: Address,
    pub slasher_implementation: Address,
    pub strategies: HashMap<String, Address>,
    pub strategy_manager: Address,
    pub strategy_manager_implementation: Address,
    pub rewards_coordinator: Address,
    pub rewards_coordinator_implementation: Address,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    pub chain_id: u64,
    pub deployment_block: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    pub executor_multisig: Address,
    pub operations_multisig: Address,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FermahManifestConfig {
    pub addresses: FermahAddresses,
    pub chain_info: ChainInfo,
    pub permissions: Permissions,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FermahAddresses {
    pub bls_apk_registry: Address,
    pub bls_apk_registry_implementation: Address,
    pub dispute_manager: Address,
    pub dispute_manager_implementation: Address,
    pub index_registry: Address,
    pub index_registry_implementation: Address,
    pub operator_state_retriever: Address,
    pub proxy_admin: Address,
    pub registry_coordinator: Address,
    pub registry_coordinator_implementation: Address,
    pub service_manager: Address,
    pub service_manager_implementation: Address,
    pub stake_registry: Address,
    pub stake_registry_implementation: Address,
    pub vault: Address,
    pub vault_implementation: Address,
    pub vault_token: Address,
    pub whitelist: Address,
    pub whitelist_implementation: Address,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub churner: Address,
    pub ejector: Address,
    pub fermah_owner: Address,
    pub fermah_upgrader: Address,
    pub pauser_registry: Address,
}
