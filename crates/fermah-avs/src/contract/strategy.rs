use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use ethers::{abi::Address, contract::abigen};

#[cfg(feature = "mock_strategy")]
use super::erc20::ERC20Mock;
use super::erc20::IERC20;
use crate::{config::Config, SignerMiddlewareContract};

abigen!(IStrategy, "contracts/out/IStrategy.sol/IStrategy.json");

#[derive(Debug, Clone)]
pub struct Strategies {
    pub strategies: HashMap<String, IStrategy<SignerMiddlewareContract>>,
    // For creation of "underlying" erc20s and mocks
    middleware: Arc<SignerMiddlewareContract>,
}

impl Strategies {
    pub fn new(config: &Config, middleware: Arc<SignerMiddlewareContract>) -> Self {
        Self {
            strategies: config
                .el_contract
                .strategies
                .iter()
                .map(|(symbol, address)| {
                    (symbol.clone(), IStrategy::new(*address, middleware.clone()))
                })
                .collect(),
            middleware: middleware.clone(),
        }
    }

    pub fn get(&self, symbol: &str) -> Option<&IStrategy<SignerMiddlewareContract>> {
        self.strategies.get(symbol)
    }

    pub async fn get_underlying(
        &self,
        symbol: &str,
    ) -> Result<Option<IERC20<SignerMiddlewareContract>>> {
        if let Some(strategy) = self.strategies.get(symbol) {
            let address: Address = strategy.underlying_token().call().await?;
            Ok(Some(IERC20::new(address, self.middleware.clone())))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "mock_strategy")]
    pub async fn get_underlying_mock(
        &self,
        symbol: &str,
    ) -> Result<Option<ERC20Mock<SignerMiddlewareContract>>> {
        if let Some(strategy) = self.strategies.get(symbol) {
            let address: Address = strategy.underlying_token().call().await?;
            Ok(Some(ERC20Mock::new(address, self.middleware.clone())))
        } else {
            Ok(None)
        }
    }
}
