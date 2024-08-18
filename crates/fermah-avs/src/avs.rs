use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use ethers::{
    providers::Middleware,
    types::{Address, TransactionReceipt, H160, U256},
};
use fermah_common::{database_path, hash::blake3::Blake3Hash, operator::OperatorId};
use tokio::{
    sync::{watch, Mutex},
    task::JoinSet,
};
use tracing::{debug, error, info, warn};

use crate::{
    contract::Contracts,
    db::{ELOperatorStatus, InMemoryAvsDb},
};

#[derive(Clone)]
pub struct Avs {
    pub contracts: Contracts,
    pub db: InMemoryAvsDb,
    pub block_number: Arc<Mutex<u64>>,
}

/// Structure that represents a request that could be associated with either address or operator id
/// The problem is that, EL contracts' methods sometimes require address of the operator, sometimes the id,
/// but we want to be able to use any. At least for now. It is usually converted to the required in the method.
/// Note: doesn't have to be an operator, but usually used for querying an operator's state
#[derive(Debug)]
pub enum OperatorRequest {
    Address(Address),
    OperatorId(OperatorId),
}

impl From<Address> for OperatorRequest {
    fn from(value: Address) -> Self {
        Self::Address(value)
    }
}

impl From<OperatorId> for OperatorRequest {
    fn from(value: OperatorId) -> Self {
        Self::OperatorId(value)
    }
}

impl From<[u8; 32]> for OperatorRequest {
    fn from(value: [u8; 32]) -> Self {
        Self::OperatorId(value.into())
    }
}

impl Avs {
    const MINIMUM_REGISTRATION_BLOCKS: u64 = 500;

    pub async fn from_contracts(contracts: Contracts) -> Result<Self> {
        let db = InMemoryAvsDb::open(database_path(".chain-sync.db"))?;
        let block_number = Arc::new(Mutex::new(0));
        Ok(Self {
            contracts,
            db,
            block_number,
        })
    }

    // fn registry_coordinator_address(&self) -> Address {
    //     self.contracts.avs_contracts.registry_coordinator.address()
    // }

    /// Get provers that are registered in EL, subtract those whos registration window is over
    pub async fn active_operator_ids(&self) -> Result<HashSet<OperatorId>> {
        let ready_provers = self.db.get_ready_provers()?;
        let current_block = { *self.block_number.lock().await };

        let ready_prover_ids = ready_provers
            .iter()
            .map(|(id, _)| id)
            .cloned()
            .collect::<HashSet<_>>();
        info!(
            ?current_block,
            ?ready_prover_ids,
            "{} ready prover(s)",
            ready_provers.len()
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
        operator: OperatorRequest,
    ) -> Result<Option<ELOperatorStatus>> {
        if let Some(operator_address) = self.operator_request_to_address(&operator)? {
            let service_manager_address: Address =
                self.contracts.fermah_contracts.service_manager.address();

            let status = self
                .contracts
                .el_contracts
                .avs_directory_storage
                .avs_operator_status(service_manager_address, operator_address)
                .call()
                .await?;
            let status = ELOperatorStatus::from(status);

            return Ok(Some(status));
        }

        Ok(None)
    }

    pub async fn operator_available_till_live(
        &self,
        operator: OperatorRequest,
    ) -> Result<Option<U256>> {
        if let Some(operator_address) = self.operator_request_to_address(&operator)? {
            let till = self
                .contracts
                .avs_contracts
                .registry_coordinator
                .registered_till_block(operator_address)
                .call()
                .await?;

            return Ok(Some(till));
        }

        Ok(None)
    }

    pub async fn operator_request_to_id(
        &self,
        operator: &OperatorRequest,
    ) -> Result<Option<OperatorId>> {
        match operator {
            OperatorRequest::Address(address) => {
                if let Some(address) = self.db.operator_address_to_id(address)? {
                    Ok(Some(address))
                } else {
                    let operator_id: OperatorId = self
                        .contracts
                        .avs_contracts
                        .registry_coordinator
                        .get_operator_id(*address)
                        .await?
                        .into();
                    if operator_id.0.is_zero() {
                        Ok(None)
                    } else {
                        self.db.register_operator_from_el(operator_id, *address)?;
                        Ok(Some(operator_id))
                    }
                }
            }
            OperatorRequest::OperatorId(operator_id) => Ok(Some(*operator_id)),
        }
    }

    pub fn operator_request_to_address(
        &self,
        operator: &OperatorRequest,
    ) -> Result<Option<Address>> {
        match operator {
            OperatorRequest::Address(address) => Ok(Some(*address)),
            OperatorRequest::OperatorId(operator_id) => self.db.operator_id_to_address(operator_id),
        }
    }

    pub async fn operator_whitelisted_config_hash(
        &self,
        operator: OperatorRequest,
    ) -> Result<Option<Blake3Hash>> {
        if let Some(operator_address) = self.operator_request_to_address(&operator)? {
            let bytes32: [u8; 32] = self
                .contracts
                .fermah_contracts
                .whitelist
                .is_whitelisted_operator(operator_address)
                .call()
                .await?;

            if bytes32 != [0; 32] {
                return Ok(Some(bytes32.into()));
            }
        }

        Ok(None)
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
        proof_requester: H160,
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
    pub async fn get_operator_balance_now(
        &self,
        proof_requester: OperatorRequest,
    ) -> Result<Option<U256>> {
        if let Some(proof_requester_address) = self.operator_request_to_address(&proof_requester)? {
            let deposit: U256 = self.get_vault_balance_now(&proof_requester_address).await?;

            Ok(Some(deposit))
        } else {
            Ok(None)
        }
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

        self.db.set_proof_requester_deposit(someone, deposit)?;
        debug!(address=?someone, ?deposit, "Checked balance");

        Ok(deposit)
    }

    pub fn get_vault_balance_cached(&self, someone: &Address) -> Option<U256> {
        self.db.get_proof_requester_deposit(someone)
    }

    // Get balance by querying the chain
    pub async fn get_operator_registered_till_now(
        &self,
        operator: OperatorRequest,
    ) -> Result<Option<U256>> {
        if let Some(operator_address) = self.operator_request_to_address(&operator)? {
            let registered_till_block: U256 = self
                .contracts
                .avs_contracts
                .registry_coordinator
                .registered_till_block(operator_address)
                .call()
                .await?;
            debug!(
                ?registered_till_block,
                ?operator_address,
                "Operator found with "
            );
            match self.operator_request_to_id(&operator).await {
                Ok(Some(operator_id)) => {
                    debug!(
                        ?registered_till_block,
                        ?operator_address,
                        ?operator_id,
                        "Operator found with id"
                    );
                    self.db
                        .set_operator_registered_till(&operator_id, registered_till_block)?;
                }
                Ok(None) => {
                    error!(?operator_address, "Operator id not found in db")
                }
                Err(err) => {
                    error!(?err, "failed to get operator id");
                    anyhow::bail!("failed to get operator id: {err:?}");
                }
            }

            Ok(Some(registered_till_block))
        } else {
            warn!(?operator, "Operator for reg block not found");
            Ok(None)
        }
    }

    pub fn get_operator_registered_till_cached(
        &self,
        operator: OperatorRequest,
    ) -> Result<Option<U256>> {
        if let Some(operator_address) = self.operator_request_to_address(&operator)? {
            Ok(self.db.get_operator_registered_till(&operator_address))
        } else {
            Ok(None)
        }
    }

    pub async fn operator_address_to_id_now(
        &self,
        operator: Address,
    ) -> Result<Option<OperatorId>> {
        let operator_id = self
            .contracts
            .avs_contracts
            .registry_coordinator
            .get_operator_id(operator)
            .call()
            .await?;
        if operator_id == [0_u8; 32] {
            Ok(None)
        } else {
            Ok(Some(operator_id.into()))
        }
    }

    pub async fn operator_id_to_address_now(
        &self,
        operator_id: OperatorId,
    ) -> Result<Option<Address>> {
        let address = self
            .contracts
            .avs_contracts
            .registry_coordinator
            .get_operator_from_id(operator_id.0 .0)
            .call()
            .await?;
        if address == Address::zero() {
            Ok(None)
        } else {
            Ok(Some(address))
        }
    }

    // pub async fn get_active_operators(&self) -> Result<Vec<OperatorId>> {
    //     self.db.get_active_operators().await
    // }

    pub async fn distribute_payments_for_many(
        &self,
        payments: &HashMap<OperatorId, HashMap<Address, U256>>,
    ) -> Result<TransactionReceipt> {
        let mut provers = vec![];
        let mut requesters = vec![];
        let mut amounts = vec![];
        for (prover, requester_amount) in payments {
            let prover = self
                .db
                .operator_id_to_address(prover)?
                .with_context(|| format!("Prover with id {prover:?} not found in CS db",))?;
            provers.push(prover);

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

        let prover = self
            .db
            .operator_id_to_address(prover)?
            .with_context(|| format!("Prover with id {prover:?} not found in CS db",))?;
        self.contracts
            .fermah_contracts
            .vault
            .distribute_to_prover(prover, requesters, amounts)
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
                                info!(?current_block_number);
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
