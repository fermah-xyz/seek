use std::sync::Arc;

use ethers::contract::abigen;

use super::{strategy::Strategies, Config, SignerMiddlewareContract};

abigen!(
    AVSDirectory,
    "../../contracts/out/IAVSDirectory.sol/IAVSDirectory.json"
);

abigen!(
    DelegationManager,
    "../../contracts/out/IDelegationManager.sol/IDelegationManager.json"
);

abigen!(
    IStrategyManager,
    "../../contracts/out/IStrategyManager.sol/IStrategyManager.json"
);

abigen!(
    IRewardsCoordinator,
    "../../contracts/out/IRewardsCoordinator.sol/IRewardsCoordinator.json"
);

abigen!(
    AVSDirectoryStorage,
    "../../contracts/out/AVSDirectoryStorage.sol/AVSDirectoryStorage.json"
);

#[derive(Clone)]
pub struct ELContracts {
    pub avs_directory: AVSDirectory<SignerMiddlewareContract>,
    pub avs_directory_storage: AVSDirectoryStorage<SignerMiddlewareContract>,
    pub delegation: DelegationManager<SignerMiddlewareContract>,
    pub strategy_manager: IStrategyManager<SignerMiddlewareContract>,
    pub rewards_coordinator: IRewardsCoordinator<SignerMiddlewareContract>,
    pub strategies: Strategies,
}

impl ELContracts {
    pub fn new(config: &Config, middleware: Arc<SignerMiddlewareContract>) -> Self {
        Self {
            avs_directory: AVSDirectory::new(config.el_contract.avs_directory, middleware.clone()),
            avs_directory_storage: AVSDirectoryStorage::new(
                config.el_contract.avs_directory,
                middleware.clone(),
            ),
            delegation: DelegationManager::new(
                config.el_contract.delegation_manager,
                middleware.clone(),
            ),
            strategy_manager: IStrategyManager::new(
                config.el_contract.strategy_manager,
                middleware.clone(),
            ),
            rewards_coordinator: IRewardsCoordinator::new(
                config.el_contract.rewards_coordinator,
                middleware.clone(),
            ),
            strategies: Strategies::new(config, middleware),
        }
    }
}
