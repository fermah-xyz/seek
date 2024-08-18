use std::{collections::HashSet, net::SocketAddr, path::Path};

use anyhow::{Context, Result};
use ethers::types::{Address, U256};
use fermah_common::operator::OperatorId;
use serde::{Deserialize, Serialize};
use sled::{IVec, Tree};
use tracing::{debug, error};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct OperatorParams {
    pub socket: Option<SocketAddr>,
    pub public_key: Option<Address>,
    pub is_el_registered: bool,

    pub registered_till_block: U256,
    // Operator status
}

impl From<OperatorParams> for IVec {
    fn from(value: OperatorParams) -> Self {
        bincode::serialize(&value).unwrap().into()
    }
}

impl From<IVec> for OperatorParams {
    fn from(value: IVec) -> Self {
        bincode::deserialize(&value).unwrap()
    }
}

impl From<&[u8]> for OperatorParams {
    fn from(value: &[u8]) -> Self {
        bincode::deserialize(value).unwrap()
    }
}

impl OperatorParams {
    pub fn is_available(&self) -> bool {
        self.socket.is_some() && self.is_el_registered
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequesterInfo {
    pub address: Address,
    pub deposit: U256,
}

impl From<IVec> for ProofRequesterInfo {
    fn from(value: IVec) -> Self {
        bincode::deserialize(&value).unwrap()
    }
}

impl From<ProofRequesterInfo> for IVec {
    fn from(value: ProofRequesterInfo) -> Self {
        bincode::serialize(&value).unwrap().into()
    }
}

impl From<&[u8]> for ProofRequesterInfo {
    fn from(value: &[u8]) -> Self {
        bincode::deserialize(value).unwrap()
    }
}

impl From<ProofRequesterInfo> for Vec<u8> {
    fn from(value: ProofRequesterInfo) -> Self {
        bincode::serialize(&value).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryAvsDb {
    // operators: Arc<Mutex<HashMap<OperatorId, OperatorParams>>>,
    operators: Tree,
    proof_requesters: Tree,
    // op_address_to_id: Arc<Mutex<HashMap<OperatorId, Address>>>,
    // op_statuses: Arc<Mutex<HashMap<OperatorId, OperatorStatus>>>,
    // op_balances: Arc<Mutex<HashMap<Address, U256>>>,
}

impl InMemoryAvsDb {
    const DB_CACHE_CAPACITY: u64 = 10 * 1024 * 1024;

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::Config::default()
            .path(&path)
            .flush_every_ms(Some(1000))
            .cache_capacity(Self::DB_CACHE_CAPACITY)
            .open()
            .inspect_err(|e| error!("sled err: {e}"))
            .with_context(|| format!("failed to open sled db: {}", path.as_ref().display()))?;

        let operators = db
            .open_tree("operators")
            .context("failed to open `operators` tree in sled DB")?;
        let proof_requesters = db
            .open_tree("proof_requesters")
            .context("failed to open `proof_requesters` tree in sled DB")?;
        Ok(Self {
            operators,
            proof_requesters,
        })
    }

    /// Function in DB, which checks if the operator is in the system, returning `Some(..)`, and if it is available `Some(true)`
    pub fn is_existing_prover(&self, operator_id: &OperatorId) -> Result<Option<bool>> {
        Ok(self
            .operators
            .iter()
            .map(|kv| {
                let (k, v) = kv.unwrap();
                (OperatorId::from(k), OperatorParams::from(v))
            })
            .find(|(o_id, _)| o_id == operator_id)
            .map(|(_, o)| o.is_available()))
    }

    pub fn get_ready_provers(&self) -> Result<HashSet<(OperatorId, OperatorParams)>> {
        Ok(self
            .operators
            .iter()
            .map(|kv| {
                let (k, v) = kv.unwrap();
                (OperatorId::from(k), OperatorParams::from(v))
            })
            .filter(|(_, o)| o.is_available())
            .collect())
    }

    pub fn register_operator_from_el(
        &self,
        operator_id: OperatorId,
        public_key: Address,
    ) -> Result<()> {
        self.operators
            .fetch_and_update(operator_id, |old| {
                match old {
                    Some(bytes) => {
                        let mut params = OperatorParams::from(bytes);
                        params.public_key = Some(public_key);
                        params.is_el_registered = true;
                        Some(params)
                    }
                    None => {
                        Some(OperatorParams {
                            public_key: Some(public_key),
                            is_el_registered: true,
                            ..Default::default()
                        })
                    }
                }
            })
            .map(|_| ())
            .context("failed to register operator from EL")
    }

    pub fn deregister_operator_from_el(
        &self,
        operator_id: &OperatorId,
        public_key: Address,
    ) -> Result<()> {
        self.operators
            .fetch_and_update(operator_id, |old| {
                match old {
                    Some(bytes) => {
                        let mut params = OperatorParams::from(bytes);
                        assert_eq!(
                            Some(public_key),
                            params.public_key,
                            "Multiple public keys for one operator is not supported"
                        );
                        params.is_el_registered = false;
                        Some(params)
                    }
                    None => None,
                }
            })
            .map(|_| ())
            .context("failed to deregister operator from EL")
    }

    pub fn operator_update_socket(
        &self,
        operator_id: OperatorId,
        socket: SocketAddr,
    ) -> Result<()> {
        self.operators
            .fetch_and_update(operator_id, |old| {
                match old {
                    Some(bytes) => {
                        let mut params = OperatorParams::from(bytes);
                        params.socket = Some(socket);
                        Some(params)
                    }
                    None => {
                        Some(OperatorParams {
                            socket: Some(socket),
                            ..Default::default()
                        })
                    }
                }
            })
            .map(|_| ())
            .context("failed to operator update socket")
    }

    pub fn operator_id_to_address(&self, operator_id: &OperatorId) -> Result<Option<Address>> {
        Ok(self
            .operators
            .get(operator_id)
            .with_context(|| format!("failed to find operator{operator_id}"))?
            .and_then(|op|
                // todo?: if public key is not present we might also return an error
                OperatorParams::from(op).public_key))
    }

    pub fn operator_address_to_id(&self, operator_address: &Address) -> Result<Option<OperatorId>> {
        Ok(self
            .operators
            .iter()
            .map(|kv| {
                let (k, v) = kv.unwrap();
                (OperatorId::from(k), OperatorParams::from(v))
            })
            .find(|(_, o)| o.public_key.as_ref() == Some(operator_address))
            .map(|(id, _)| id))
    }

    pub fn get_proof_requester_deposit(&self, proof_requester: &Address) -> Option<U256> {
        self.proof_requesters.get(proof_requester).ok()?.map(|v| {
            let prer = ProofRequesterInfo::from(v);
            prer.deposit
        })
    }

    // TODO!: we create a PRer here, prob not correct and need a proper PRer creation
    pub fn set_proof_requester_deposit(
        &self,
        proof_requester: &Address,
        deposit: U256,
    ) -> Result<()> {
        self.proof_requesters
            .fetch_and_update(proof_requester.as_bytes(), |v| {
                if let Some(v) = v {
                    let mut prer = ProofRequesterInfo::from(v);
                    prer.deposit = deposit;
                    Some(prer)
                } else {
                    Some(ProofRequesterInfo {
                        address: *proof_requester,
                        deposit,
                    })
                }
            })?;

        Ok(())
    }

    pub fn set_operator_registered_till(
        &self,
        operator: &OperatorId,
        registered_till_block: U256,
    ) -> Result<()> {
        self.operators.fetch_and_update(operator, |v| {
            if let Some(v) = v {
                debug!(?registered_till_block, ?operator, "Updating reg till");

                let mut prer = OperatorParams::from(v);
                prer.registered_till_block = registered_till_block;
                Some(prer)
            } else {
                debug!(?registered_till_block, ?operator, "Operator not found");
                None
            }
        })?;

        Ok(())
    }

    pub fn get_operator_registered_till(&self, proof_requester: &Address) -> Option<U256> {
        self.proof_requesters
            .get(proof_requester.as_bytes())
            .ok()?
            .map(|v| {
                let prer = OperatorParams::from(v);
                prer.registered_till_block
            })
    }
}
