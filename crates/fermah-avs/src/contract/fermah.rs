use std::sync::Arc;

use ethers::contract::abigen;

#[cfg(feature = "mock_vault_token")]
use super::erc20::ERC20Mock;
#[cfg(not(feature = "mock_vault_token"))]
use super::erc20::IERC20;
use super::{Config, SignerMiddlewareContract};

abigen!(
    ServiceManager,
    "../../contracts/out/IServiceManager.sol/IServiceManager.json"
);

abigen!(
    DisputeManager,
    "../../contracts/out/DisputeManager.sol/DisputeManager.json"
);

abigen!(Vault, "../../contracts/out/Vault.sol/Vault.json");

abigen!(
    Whitelist,
    "../../contracts/out/Whitelist.sol/Whitelist.json"
);

#[derive(Debug, Clone)]
pub struct FermahContracts {
    pub service_manager: ServiceManager<SignerMiddlewareContract>,
    pub dispute_manager: DisputeManager<SignerMiddlewareContract>,
    pub vault: Vault<SignerMiddlewareContract>,
    pub whitelist: Whitelist<SignerMiddlewareContract>,

    #[cfg(not(feature = "mock_vault_token"))]
    pub vault_token: IERC20<SignerMiddlewareContract>,
    #[cfg(feature = "mock_vault_token")]
    pub vault_token: ERC20Mock<SignerMiddlewareContract>,
}

impl FermahContracts {
    pub fn new(config: &Config, middleware: Arc<SignerMiddlewareContract>) -> Self {
        Self {
            service_manager: ServiceManager::new(
                config.fermah_contract.service_manager,
                middleware.clone(),
            ),
            dispute_manager: DisputeManager::new(
                config.fermah_contract.dispute_manager,
                middleware.clone(),
            ),
            vault: Vault::new(config.fermah_contract.vault, middleware.clone()),
            whitelist: Whitelist::new(config.fermah_contract.whitelist, middleware.clone()),

            #[cfg(not(feature = "mock_vault_token"))]
            vault_token: IERC20::new(config.fermah_contract.vault_token, middleware),
            #[cfg(feature = "mock_vault_token")]
            vault_token: ERC20Mock::new(config.fermah_contract.vault_token, middleware),
        }
    }
}
