use std::sync::Arc;

use ethers::contract::abigen;

use super::{Config, SignerMiddlewareContract};

abigen!(
    FermahRegistryCoordinator,
    "contracts/out/FermahRegistryCoordinator.sol/FermahRegistryCoordinator.json"
);

abigen!(
    OperatorStateRetriever,
    "contracts/out/OperatorStateRetriever.sol/OperatorStateRetriever.json"
);

#[derive(Debug, Clone)]
pub struct AVSContracts {
    pub registry_coordinator: FermahRegistryCoordinator<SignerMiddlewareContract>,
    pub operator_state_retriever: OperatorStateRetriever<SignerMiddlewareContract>,
}

impl AVSContracts {
    pub fn new(config: &Config, middleware: Arc<SignerMiddlewareContract>) -> Self {
        Self {
            registry_coordinator: FermahRegistryCoordinator::new(
                config.avs_contract.registry_coordinator,
                middleware.clone(),
            ),
            operator_state_retriever: OperatorStateRetriever::new(
                config.avs_contract.operator_state_retriever,
                middleware.clone(),
            ),
        }
    }
}
