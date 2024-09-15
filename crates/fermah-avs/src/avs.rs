#[cfg(feature = "db")]
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use ethers::{
    providers::Middleware,
    types::{Address, TransactionReceipt, U256},
};
use fermah_common::{
    hash::{blake3::Blake3Hash, keccak256::Keccak256Hash},
    operator::OperatorId,
};
#[cfg(feature = "db")]
use fermah_database::Database;
use tokio::{
    sync::{watch, Mutex},
    task::JoinSet,
};
use tracing::{debug, info, warn};

use crate::{contract::Contracts, ELOperatorStatus};

#[derive(Clone)]
pub struct Avs {
    pub contracts: Contracts,
    #[cfg(feature = "db")]
    pub database: Database,
    pub block_number: Arc<Mutex<u64>>,
}

impl Avs {
    const MINIMUM_REGISTRATION_BLOCKS: u64 = 500;

    pub async fn from_contracts(
        contracts: Contracts,
        #[cfg(feature = "db")] database: Database,
    ) -> Result<Self> {
        let block_number = Arc::new(Mutex::new(0));
        Ok(Self {
            contracts,
            #[cfg(feature = "db")]
            database,
            block_number,
        })
    }

    pub async fn check_operator(&self, operator_id: &OperatorId) -> Result<Option<Keccak256Hash>> {
        let el_operator_id: Keccak256Hash = self
            .contracts
            .avs_contracts
            .registry_coordinator
            .get_operator_id(operator_id.0)
            .await?
            .into();

        if el_operator_id.0.is_zero() {
            Ok(None)
        } else {
            #[cfg(feature = "db")]
            self.database.register_operator_from_el(*operator_id)?;
            Ok(Some(el_operator_id))
        }
    }

    /// Gets raw registeredTillBlock for an operator. Important that as raw request, so it returns 0, for instance
    /// if operator is not registered. This means that, unlike some methods that return an Option<T> where None signals that
    /// operator is not registered, this method doesn't distinguish between operators which are actually registered and not.
    pub async fn get_registered_till_block(&self, operator_id: &OperatorId) -> Result<U256> {
        let registered_till_block: U256 = self
            .contracts
            .avs_contracts
            .registry_coordinator
            .registered_till_block(operator_id.0)
            .call()
            .await?;

        #[cfg(feature = "db")]
        self.database
            .set_operator_registered_till(operator_id, registered_till_block)?;

        Ok(registered_till_block)
    }

    pub async fn check_registered_till_block(&self, operator_id: &OperatorId) -> Result<bool> {
        let registered_till_block = self.get_registered_till_block(operator_id).await?;
        let current_block: U256 = { *self.block_number.lock().await }.into();

        Ok(current_block + Self::MINIMUM_REGISTRATION_BLOCKS < registered_till_block)
    }

    // fn registry_coordinator_address(&self) -> Address {
    //     self.contracts.avs_contracts.registry_coordinator.address()
    // }

    #[cfg(feature = "db")]
    /// Get provers that are registered in EL, subtract those whos registration window is over
    pub async fn active_operator_ids(&self) -> Result<HashSet<OperatorId>> {
        let ready_provers = self.database.get_ready_provers()?;
        let current_block = { *self.block_number.lock().await };

        let ready_prover_ids = ready_provers
            .iter()
            .map(|(id, _)| id)
            .cloned()
            .collect::<HashSet<_>>();
        debug!(
            ?ready_prover_ids,
            provers_count = ready_provers.len(),
            "ready provers",
        );

        let safe_block: U256 = (current_block + Self::MINIMUM_REGISTRATION_BLOCKS).into();

        Ok(ready_provers
            .iter()
            .filter_map(|(id, operator)| {
                if operator.registered_till_block >= safe_block {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect())
    }

    pub async fn el_operator_status_live(
        &self,
        operator_id: &OperatorId,
    ) -> Result<ELOperatorStatus> {
        let service_manager_address: Address =
            self.contracts.fermah_contracts.service_manager.address();

        let status = self
            .contracts
            .el_contracts
            .avs_directory_storage
            .avs_operator_status(service_manager_address, operator_id.0)
            .call()
            .await?;
        let status = ELOperatorStatus::from(status);

        Ok(status)
    }

    pub async fn operator_available_till_live(&self, operator_id: OperatorId) -> Result<U256> {
        let till = self
            .contracts
            .avs_contracts
            .registry_coordinator
            .registered_till_block(operator_id.0)
            .call()
            .await?;

        Ok(till)
    }

    pub async fn operator_whitelisted_config_hash(
        &self,
        operator_id: OperatorId,
    ) -> Result<Option<Blake3Hash>> {
        let bytes32: [u8; 32] = self
            .contracts
            .fermah_contracts
            .whitelist
            .is_whitelisted_operator(operator_id.0)
            .call()
            .await?;

        if bytes32 != [0; 32] {
            Ok(Some(bytes32.into()))
        } else {
            Ok(None)
        }
    }

    // pub async fn operator_status(
    //     &self,
    //     operator: OperatorRequest,
    // ) -> Result<Option<OperatorStatus>> {
    //     if let Some(operator_id) = self.operator_request_to_id(operator).await? {
    //         if let Some(status) = self.db.get_operator_status(&operator_id).await? {
    //             return Ok(Some(status));
    //         }
    //     }
    //     Ok(None)
    // }

    // This function is not quite finished and doesn't do what it is intended for, for 100%, but has a code that will be helpful later
    //pub async fn operator_active_windows(
    //    &self,
    //    operator: OperatorRequest,
    //    block_number: u64,
    //) -> Result<bool> {
    //    if let Some(operator_id) = self.operator_request_to_id(operator).await? {
    //        let _state: (
    //            U256,
    //            Vec<Vec<bindings::avs::contracts::operator_state_retriever::Operator>>,
    //        ) = self
    //            .contracts
    //            .avs_contracts
    //            .operator_state_retriever
    //            .get_operator_state_with_registry_coordinator_and_operator_id(
    //                Address::from(self.registry_coordinator_address()),
    //                operator_id.into(),
    //                block_number.try_into().expect("We will never get there"),
    //            )
    //            .call()
    //            .await
    //            .unwrap();

    //        return Ok(true);
    //    }

    //    Ok(false)
    //}

    // pub async fn reserve(
    //     &self,
    //     proof_requester: Address,
    //     amount: U256,
    // ) -> Result<TransactionReceipt> {
    //     self.contracts
    //         .fermah_contracts
    //         .vault
    //         .reserve(amount, proof_requester)
    //         .send()
    //         .await?
    //         .await?
    //         .context("failed to reserve")
    // }

    // pub async fn unreserve(
    //     &self,
    //     proof_requester: H160,
    //     amount: U256,
    // ) -> Result<TransactionReceipt> {
    //     self.contracts
    //         .fermah_contracts
    //         .vault
    //         .unreserve(amount, proof_requester)
    //         .send()
    //         .await?
    //         .await?
    //         .context("failed to reserve")
    // }

    pub async fn withdraw_to_requester(
        &self,
        proof_requester: Address,
        amount: U256,
    ) -> Result<TransactionReceipt> {
        self.contracts
            .fermah_contracts
            .vault
            .withdraw(amount, proof_requester)
            .send()
            .await?
            .await?
            .context("failed to reserve")
    }

    // Get balance by querying the chain
    pub async fn get_vault_balance_now(&self, someone: &Address) -> Result<U256> {
        let deposit: U256 = self
            .contracts
            .fermah_contracts
            .vault
            .balances(*someone)
            .call()
            .await?;

        #[cfg(feature = "db")]
        if !deposit.is_zero() {
            self.database
                .set_proof_requester_deposit(someone, deposit)?;
        }

        debug!(address=?someone, ?deposit, "Checked balance");

        Ok(deposit)
    }

    #[cfg(feature = "db")]
    pub fn get_vault_balance_cached(&self, someone: &Address) -> Result<Option<U256>> {
        self.database.get_seeker_deposit(someone)
    }

    pub async fn get_operator_registered_till_now(
        &self,
        operator_id: OperatorId,
    ) -> Result<Option<U256>> {
        let registered_till_block: U256 = self
            .contracts
            .avs_contracts
            .registry_coordinator
            .registered_till_block(operator_id.0)
            .call()
            .await?;
        debug!(
            ?registered_till_block,
            operator_address = ?operator_id.0,
            "Operator found with "
        );

        #[cfg(feature = "db")]
        self.database
            .set_operator_registered_till(&operator_id, registered_till_block)?;

        if registered_till_block.is_zero() {
            return Ok(None);
        }
        Ok(Some(registered_till_block))
    }

    pub async fn distribute_payments_for_many(
        &self,
        payments: &HashMap<OperatorId, HashMap<Address, U256>>,
    ) -> Result<TransactionReceipt> {
        let mut provers = vec![];
        let mut requesters = vec![];
        let mut amounts = vec![];
        for (prover, requester_amount) in payments {
            provers.push(prover.0);

            let mut reqs = vec![];
            let mut amts = vec![];
            for (requester, amount) in requester_amount {
                reqs.push(*requester);
                amts.push(*amount);
            }
            requesters.push(reqs);
            amounts.push(amts);
        }
        self.contracts
            .fermah_contracts
            .vault
            .distribute_to_provers(provers, requesters, amounts)
            .send()
            .await?
            .await?
            .context("failed to distribute")
    }

    pub async fn distribute_payments(
        &self,
        prover: &OperatorId,
        payments: &HashMap<Address, U256>,
    ) -> Result<TransactionReceipt> {
        let mut requesters = vec![];
        let mut amounts = vec![];
        for (proof_requester, requester_amount) in payments {
            requesters.push(*proof_requester);
            amounts.push(*requester_amount);
        }

        self.contracts
            .fermah_contracts
            .vault
            .distribute_to_prover(prover.0, requesters, amounts)
            .send()
            .await?
            .await?
            .context("failed to distribute")
    }

    const HOLESKY_SLOT_DURATION: Duration = Duration::from_secs(12);
    /// A block is minted every 12 seconds on the Holesky network.
    /// TODO: use websocket for mainnet.
    pub async fn start_holesky_block_update_thread(
        &self,
        tasks: &mut JoinSet<Result<()>>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        let provider = self.contracts.provider.clone();
        let block_number = self.block_number.clone();
        let mut interval = tokio::time::interval(Self::HOLESKY_SLOT_DURATION);
        tasks.spawn({
            async move {
                loop {
                    tokio::select! {
                        _ = shutdown_rx.changed() => {
                            info!("Block update thread stopped");
                            return Ok(())
                        }

                        _= interval.tick() => {
                            if let Ok(current_block_number) = provider.get_block_number().await {
                                let mut block_lock = block_number.lock().await;
                                *block_lock = current_block_number.as_u64();
                            } else {
                                warn!("failed to read current block number");
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }
}
