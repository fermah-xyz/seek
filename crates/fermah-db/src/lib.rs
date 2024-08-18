use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::{bail, Context, Result};
use blake3::Hash;
use chrono::{DateTime, Utc};
use ethers::types::{Address, U256};
use fermah_common::{
    crypto::signer::{ecdsa::EcdsaSigner, SignedData},
    hash::blake3::Blake3Hash,
    operator::OperatorId,
    proof::{
        request::{ProofRequest, ProofRequestId},
        status::ProofStatus,
    },
    resource::Resource,
};
use serde::{Deserialize, Serialize};
use sled::{transaction::ConflictableTransactionError, IVec, Tree};
use tracing::error;

pub fn hash_from_ivec(value: IVec) -> Hash {
    bincode::deserialize(&value).unwrap()
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Payment {
    #[default]
    Nothing,
    // Amount of money to be reserved from the proof requester
    ToReserve(U256),
    // We have reserved this much for the proof generation task, preliminary blocked from ProofRequester's account
    // note: will start reserved until the MM receives the proof, or it is obvious that Prover needs to be paid
    //       or we unreserve it in case we cannot fulfill the PR
    Reserved(U256),
    // All the work is done, and the reserved amount is ready to be paid to the proof requester
    ReadyToPay(U256),
    // We have paid this much for the proof generation task from ProofRequester's account
    Paid(U256),
    // Not used, but supposed to be at some point
    Refund(U256),
}

// todo?: I know that the operator_id is already is the key in the InMemoryDBInner.operators, but for certain operations on the OperatorInfo
//        it would be great to be able to have that operator_id ready. If there is a better way to handle it in `available_operators`, then we could refactor it later
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorInfo {
    pub operator_id: OperatorId,
    pub resource: Resource,
    // i64 because it can go to negative
    pub reputation: i64,
    /// Last recorded interaction with the operator
    pub last_interaction: DateTime<Utc>,
    /// If operator is online. Use with caution! Usually should be proactively updated.
    /// Will be changed to `false` if operator gracefully (presumably temporarily) exits.
    /// Will be overwritten to `true` with the whole structure when `NewConnection` from P2P.
    pub online: bool,
}

impl OperatorInfo {
    /// Checks if the operator is online
    pub fn is_online(&self) -> bool {
        self.online && (Utc::now() - self.last_interaction).num_minutes() < 3
    }

    /// Checks if the operator is registered as online, but has not sent any message for last 2 mins
    pub fn is_temporary_offline(&self) -> bool {
        self.online && (Utc::now() - self.last_interaction).num_minutes() >= 3
    }
}

impl From<IVec> for OperatorInfo {
    fn from(value: IVec) -> Self {
        bincode::deserialize(&value).unwrap()
    }
}

impl From<OperatorInfo> for IVec {
    fn from(value: OperatorInfo) -> Self {
        bincode::serialize(&value).unwrap().into()
    }
}

impl From<&[u8]> for OperatorInfo {
    fn from(value: &[u8]) -> Self {
        bincode::deserialize(value).unwrap()
    }
}

impl From<OperatorInfo> for Vec<u8> {
    fn from(value: OperatorInfo) -> Self {
        bincode::serialize(&value).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequestParams {
    pub signed_payload: SignedData<ProofRequest, EcdsaSigner>,
    pub assigned: Option<OperatorId>,
    pub status: ProofStatus,
    pub last_status_update: DateTime<Utc>,
    pub payment: Payment,
}

impl ProofRequestParams {
    pub fn created(signed_payload: SignedData<ProofRequest, EcdsaSigner>) -> Self {
        Self {
            signed_payload,
            assigned: None,
            status: ProofStatus::Created,
            last_status_update: Utc::now(),
            payment: Payment::Nothing,
        }
    }

    /// Function that tells if there are any money that should be witheld from returning to the proof requester
    // note: this doesn't take into account those PRs that were processed, but due to unsatisfactory results PRer's funds could be returned
    // note: this generally doesn't take into account another field `status`, with which, Params should have status and payment merged somehow
    pub fn not_elighable_for_returns(&self) -> Option<U256> {
        match self.payment {
            Payment::Refund(_amount) => None,
            Payment::ReadyToPay(amount) => Some(amount),
            Payment::Reserved(amount) => Some(amount),
            // We ignore paid, because it is supposedly paid already and not accessible
            Payment::Paid(_) => None,
            // This supposed to be recoverable.
            Payment::ToReserve(_) => None,
            Payment::Nothing => None,
        }
    }
}

impl From<IVec> for ProofRequestParams {
    fn from(value: IVec) -> Self {
        bincode::deserialize(&value).unwrap()
    }
}
impl From<&[u8]> for ProofRequestParams {
    fn from(value: &[u8]) -> Self {
        bincode::deserialize(value).unwrap()
    }
}

impl From<ProofRequestParams> for IVec {
    fn from(value: ProofRequestParams) -> Self {
        bincode::serialize(&value).unwrap().into()
    }
}

impl From<ProofRequestParams> for Vec<u8> {
    fn from(value: ProofRequestParams) -> Self {
        bincode::serialize(&value).unwrap()
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryDBInner {
    pub proof_requests:
        HashMap<ProofRequestId, (SignedData<ProofRequest, EcdsaSigner>, ProofStatus, Payment)>,
    // pub operators: HashMap<OperatorId, OperatorInfo>,
}

#[derive(Debug, Clone)]
// Very dirty, of course, but we can think of every access as a database transaction
pub struct InMemoryDB {
    // Even for dirty impl RwLock in place of mutex probably would be better,
    // since we don't write on every access
    //  inner: Arc<RwLock<InMemoryDBInner>>,
    operators: Tree,
    proof_requests: Tree,
}

impl InMemoryDB {
    const DB_CACHE_CAPACITY: u64 = 10 * 1024 * 1024;
    const REASSIGNMENT_SECONDS: u64 = 10;
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::Config::default()
            .path(&path)
            .flush_every_ms(Some(1000))
            .cache_capacity(Self::DB_CACHE_CAPACITY)
            .open()
            .with_context(|| format!("failed to open sled db: {}", path.as_ref().display()))?;

        let operators = db
            .open_tree("operators")
            .context("failed to open `operators` tree in sled DB")?;
        let proof_requests = db
            .open_tree("proof_requests")
            .context("failed to open `proof_requests` tree in sled DB")?;
        Ok(Self {
            operators,
            proof_requests,
        })
    }

    /// Returns operators that are not occupied by any tasks
    pub fn available_operators(&self) -> Vec<OperatorInfo> {
        let occupied_operators: HashSet<OperatorId> = self
            .proof_requests
            .iter()
            .filter_map(|r| {
                if r.is_err() {
                    tracing::warn!(?r, "occupied_operators error");
                }
                r.ok()
            })
            .filter_map(|(_k, v)| {
                let params = ProofRequestParams::from(v);
                match params.status {
                    ProofStatus::Assigned(prover) => {
                        tracing::info!(?prover, "prover Assigned");
                        Some(prover)
                    }
                    ProofStatus::AcknowledgedAssignment(prover) => {
                        tracing::info!(?prover, "prover AcknowledgedAssignment");
                        Some(prover)
                    }
                    _ => None,
                }
            })
            .collect();

        self.operators
            .iter()
            .filter_map(|kv| {
                let kv = kv.ok()?;
                let operator_id = kv.0.into();
                let operator_info: OperatorInfo = kv.1.into();
                if occupied_operators.contains(&operator_id) || !operator_info.is_online() {
                    None
                } else {
                    Some(operator_info)
                }
            })
            .collect()
    }

    pub fn register_operator_from_p2p(
        &self,
        operator_id: OperatorId,
        resource: Resource,
    ) -> Result<()> {
        let info = OperatorInfo {
            operator_id,
            reputation: 0,
            resource,
            last_interaction: Utc::now(),
            online: true,
        };
        self.operators
            .insert(operator_id, info)
            .map(|_| ())
            .context("failed to register operator from P2P")
    }

    pub fn unregister_operator_from_p2p(&self, operator_id: &OperatorId) -> Result<()> {
        self.operators
            .remove(operator_id)
            .map(|_| ())
            .context("failed to unregister operator from P2P")
    }

    pub fn set_proof_request_status(
        &self,
        proof_request_id: &ProofRequestId,
        status: ProofStatus,
    ) -> Result<()> {
        self.proof_requests
            .fetch_and_update(proof_request_id.as_ref(), |old| {
                match old {
                    Some(bytes) => {
                        let mut params = ProofRequestParams::from(bytes);
                        params.last_status_update = Utc::now();
                        if let ProofStatus::Assigned(operator_id) = status {
                            params.assigned = Some(operator_id);
                        }
                        params.status = status.clone();
                        Some(params)
                    }
                    None => None,
                }
            })
            .map(|_| ())
            .context("failed to set proof request status")
    }

    pub fn set_payment_status(
        &self,
        proof_request_id: &ProofRequestId,
        payment_status: Payment,
    ) -> Result<()> {
        self.proof_requests
            .fetch_and_update(proof_request_id.as_ref(), |old| {
                match old {
                    Some(bytes) => {
                        let mut params = ProofRequestParams::from(bytes);
                        params.payment = payment_status;
                        Some(params)
                    }
                    None => None,
                }
            })
            .map(|_| ())
            .context("failed to set proof request payment status")
    }

    pub fn set_payment_to_ready(&self, proof_request_id: &ProofRequestId) -> Result<()> {
        let mut incorrect_status = None;
        self.proof_requests
            .fetch_and_update(proof_request_id.as_ref(), |old| {
                match old {
                    Some(bytes) => {
                        let mut params = ProofRequestParams::from(bytes);
                        if let Payment::Reserved(reserved_amount) = params.payment {
                            params.payment = Payment::ReadyToPay(reserved_amount);
                        } else {
                            incorrect_status = Some(params.payment);
                        }

                        Some(params)
                    }
                    None => None,
                }
            })
            .map(|_| ())
            .context("failed to set proof request payment status")?;
        if let Some(payment_status) = incorrect_status {
            bail!(
                "Expected payment status of proof request {:?} to be Reserved, but is {:?}",
                proof_request_id,
                payment_status
            );
        } else {
            Ok(())
        }
    }

    pub fn get_reserved_for_requester(&self, proof_requester: Address) -> U256 {
        self.proof_requests
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_k, v)| {
                let params = ProofRequestParams::from(v);
                match (params.signed_payload.public_key, params.payment) {
                    (addr, Payment::Reserved(reserved)) if addr == proof_requester => {
                        Some(reserved)
                    }
                    _ => None,
                }
            })
            .fold(U256::zero(), |acc, e| acc + e)
    }

    pub fn try_create_proof_request(
        &self,
        proof_request: SignedData<ProofRequest, EcdsaSigner>,
    ) -> Result<Blake3Hash> {
        let proof_request_id = proof_request.hash;

        if self
            .proof_requests
            .get(proof_request_id.as_ref())?
            .is_some()
        {
            bail!("Proof request exists");
        }

        let _old = self
            .proof_requests
            .insert(
                proof_request_id.as_ref(),
                ProofRequestParams::created(proof_request),
            )
            .context("failed to create proof request")?;

        // todo: doublecheck: the line below is not correct to be here, because we don't want to create a new PR in the db, removing old one's state
        //       What we want is to check in advance and just warn internally.
        // anyhow::ensure!(old.is_none(), "proof request already exists");
        Ok(proof_request_id)
    }

    // note: We use SignedData<ProofRequest, EthSigner>, and not the PR itself, because particularly SignedData<ProofRequest, EthSigner> provides the `.id()`
    //       method for PR
    // todo: Ideally it should also include some metadata, such as timestamp of when we acknowledged the PR, so that we can
    //       prioritize PRs, and also discard them if they
    /// Proof requests that are ready for assignment. Note: requests, that were not Acknowledged for N seconds, are also returned for reassignment
    pub fn proof_requests_need_assignment(
        &self,
    ) -> Result<Vec<SignedData<ProofRequest, EcdsaSigner>>> {
        Ok(self
            .proof_requests
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_k, v)| {
                let params = ProofRequestParams::from(v);
                match params.status {
                    ProofStatus::Accepted => Some(params.signed_payload.clone()),
                    ProofStatus::Assigned(_) => {
                        let delta = Utc::now() - params.last_status_update;
                        let delta_seconds = delta.num_seconds();

                        if delta_seconds > 0 && delta_seconds as u64 >= Self::REASSIGNMENT_SECONDS {
                            Some(params.signed_payload.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect())
    }

    pub fn set_proof_requests_paid(&self, proof_request_ids: &Vec<ProofRequestId>) -> Result<()> {
        // TODO: handle errors in the whold function
        if let Err(e) = self.proof_requests.transaction(move |tx_db| {
            for proof_request_id in proof_request_ids {
                let value = tx_db
                    .get(proof_request_id.as_ref())
                    .expect("Should be there")
                    .expect("Suppose to be there, unless race condition");
                let mut params = ProofRequestParams::from(value);
                if let Payment::ReadyToPay(amount) = params.payment {
                    params.payment = Payment::Paid(amount);
                    tx_db
                        .insert(proof_request_id.as_ref(), params)
                        .expect("todo");
                } else {
                    // todo: handle
                }
            }

            Ok::<(), ConflictableTransactionError<anyhow::Error>>(())
        }) {
            bail!("failed to update proof requests to paid: {:?}", e)
        }

        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn get_ready_to_pay_proof_requests_for_many(
        &self,
    ) -> Result<(
        HashMap<OperatorId, HashMap<Address, U256>>,
        Vec<ProofRequestId>,
    )> {
        let prs: Vec<ProofRequestParams> = self
            .proof_requests
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_k, v)| {
                let params = ProofRequestParams::from(v);
                match params.payment {
                    Payment::ReadyToPay(_) => Some(params.clone()),
                    _ => None,
                }
            })
            .collect();

        let mut payments: HashMap<OperatorId, HashMap<Address, U256>> = HashMap::new();
        let mut to_be_paid = vec![];

        for pr in prs.into_iter() {
            let prover = if let Some(prover) = pr.assigned {
                prover
            } else {
                bail!("No Prover assigned, but Proof request is ready to pay. How?")
            };
            let requester = pr.signed_payload.payload.requester.unwrap();
            if let Payment::ReadyToPay(amount) = pr.payment {
                if let Some(p) = payments.get_mut(&prover) {
                    if let Some(to_pay) = p.get_mut(&requester) {
                        if to_pay.checked_add(amount).is_none() {
                            // todo: finish it
                            bail!("Overflow occured")
                        }
                    } else {
                        p.insert(requester, amount);
                    }
                } else {
                    payments.insert(prover, HashMap::from([(requester, amount)]));
                }
            }
            to_be_paid.push(pr.signed_payload.hash);
        }
        Ok((payments, to_be_paid))
    }

    // TODO: Check
    #[allow(clippy::type_complexity)]
    pub fn get_ready_to_pay_proof_requests(
        &self,
        operator_id: &OperatorId,
    ) -> Result<(HashMap<Address, U256>, Vec<ProofRequestId>)> {
        let prs: Vec<ProofRequestParams> = self
            .proof_requests
            .iter()
            .filter_map(|r| r.ok())
            .filter_map(|(_k, v): (IVec, IVec)| {
                let params = ProofRequestParams::from(v);
                if let Some(prover) = &params.assigned {
                    if prover == operator_id {
                        match params.payment {
                            Payment::ReadyToPay(_) => Some(params.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        let mut payments: HashMap<Address, U256> = HashMap::new();
        let mut to_be_paid = vec![];

        for pr in prs.into_iter() {
            let requester = pr.signed_payload.payload.requester.unwrap();
            if let Payment::ReadyToPay(amount) = pr.payment {
                if let Some(to_pay) = payments.get_mut(&requester) {
                    if to_pay.checked_add(amount).is_none() {
                        // todo: finish it
                        bail!("Overflow occured")
                    }
                } else {
                    payments.insert(requester, amount);
                }
            }
            to_be_paid.push(pr.signed_payload.hash);
        }
        Ok((payments, to_be_paid))
    }

    /// Closes existing unassigned PRs and returns amount of money which is already reserved for payment, to deduct it later.
    /// Note: possible race condition, due to lack of bulk reading in Sled transactions
    #[allow(clippy::type_complexity)]
    pub fn non_refundable_amount(&self, proof_requester: &Address) -> Result<U256> {
        Ok(self
            .proof_requests
            .iter()
            .filter_map(|r| r.ok())
            // PRs created by PRer
            .filter_map(|(_k, v): (IVec, IVec)| {
                let params = ProofRequestParams::from(v);
                if *proof_requester == params.signed_payload.public_key {
                    params.not_elighable_for_returns()
                } else {
                    None
                }
            })
            // Collect PR ids to be cancelled, and amount of money to be NOT returned
            .fold(U256::zero(), |acc, amount| {
                if let Some(acc) = acc.checked_add(amount) {
                    acc
                } else {
                    error!("Failed to reserve for not refundable");
                    U256::max_value()
                }
            }))
    }

    pub fn get_operator(&self, operator_id: &OperatorId) -> Result<Option<OperatorInfo>> {
        self.operators
            .get(operator_id)
            .context("failed to read operator")
            .map(|o| o.map(OperatorInfo::from))
    }

    pub fn get_proof_request(
        &self,
        proof_request_id: &ProofRequestId,
    ) -> Result<Option<ProofRequestParams>> {
        self.proof_requests
            .get(proof_request_id.as_ref())
            .context("failed to read proof request")
            .map(|o| o.map(ProofRequestParams::from))
    }

    pub fn update_last_interaction(&self, operator_id: &OperatorId) -> Result<()> {
        self.operators
            .fetch_and_update(operator_id, |old| {
                old.map(OperatorInfo::from).map(|mut op| {
                    op.last_interaction = Utc::now();
                    op
                })
            })
            .context("failed to fetch and update `last_interaction` for an operator")?;

        Ok(())
    }

    /// Returns an aggreagation of opeators: All in the DB, online, registered as online, but not responsive
    pub fn get_operator_counts(&self) -> Result<(u64, u64, u64)> {
        Ok(self
            .operators
            .iter()
            .filter_map(|r| r.ok())
            .map(|(_k, v): (IVec, IVec)| OperatorInfo::from(v.clone()))
            .fold(
                (0, 0, 0),
                |(all, mut online, mut temporary_offline), operator| {
                    if operator.is_online() {
                        online += 1;
                    } else if operator.online {
                        temporary_offline += 1;
                    }

                    (all + 1, online, temporary_offline)
                },
            ))
    }
}

#[cfg(test)]
mod tests {
    use ethers::core::k256::ecdsa::SigningKey;
    use fermah_common::{
        crypto::signer::Signer,
        executable::{Executable, ResultExtractor},
        hash::blake3::Blake3Hash,
        resource::requirement::ResourceRequirement,
    };
    use rand::{prelude::StdRng, SeedableRng};

    pub use super::*;

    #[test]
    fn test_merge_functionality() {
        #[derive(Serialize, Deserialize, Clone)]
        struct Smth {
            a: u8,
            b: u8,
            c: u8,
        }

        impl From<IVec> for Smth {
            fn from(value: IVec) -> Self {
                bincode::deserialize(&value).unwrap()
            }
        }

        impl From<&[u8]> for Smth {
            fn from(value: &[u8]) -> Self {
                bincode::deserialize(value).unwrap()
            }
        }

        impl From<Smth> for IVec {
            fn from(value: Smth) -> Self {
                bincode::serialize(&value).unwrap().into()
            }
        }

        impl From<Smth> for Vec<u8> {
            fn from(value: Smth) -> Self {
                bincode::serialize(&value).unwrap()
            }
        }

        let db = sled::Config::default()
            .path("/tmp/test_smth")
            .flush_every_ms(Some(1000))
            .cache_capacity(1024 * 1024 * 1337)
            .open()
            .unwrap();

        let something = db.open_tree("something").unwrap();

        something.insert("0", Smth { a: 0, b: 0, c: 3 }).unwrap();
        something.insert("1", Smth { a: 0, b: 1, c: 3 }).unwrap();

        if let Some(smth) = something.get("0").unwrap() {
            let smth = Smth::from(smth).a;
            assert_eq!(smth, 0);
        } else {
            panic!("No data found")
        }

        fn update_b(
            _key: &[u8],              // the key being merged
            old_value: Option<&[u8]>, // the previous value, if one existed
            merged_bytes: &[u8],      // the new bytes being merged in
        ) -> Option<Vec<u8>> {
            // set the new value, return None to delete
            if let Some(mut smth) = old_value.map(Smth::from) {
                smth.b = merged_bytes[0];
                Some(smth.into())
            } else {
                None
            }
        }

        fn update_c(
            _key: &[u8],              // the key being merged
            old_value: Option<&[u8]>, // the previous value, if one existed
            merged_bytes: &[u8],      // the new bytes being merged in
        ) -> Option<Vec<u8>> {
            // set the new value, return None to delete
            if let Some(mut smth) = old_value.map(Smth::from) {
                smth.c = merged_bytes[0];
                Some(smth.into())
            } else {
                None
            }
        }

        something.set_merge_operator(update_b);

        something.merge("0", [5]).unwrap();

        if let Some(smth) = something.get("0").unwrap() {
            let smth = Smth::from(smth).b;
            assert_eq!(smth, 5);
        } else {
            panic!("No data found")
        }

        something.set_merge_operator(update_c);

        something.merge("0", [8]).unwrap();

        if let Some(smth) = something.get("0").unwrap() {
            let smth = Smth::from(smth).c;
            assert_eq!(smth, 8);
        } else {
            panic!("No data found")
        }
    }

    #[test]
    fn test_serde() {
        let pr = ProofRequest {
            requester: Some(
                "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
                    .parse()
                    .unwrap(),
            ),
            prover: Executable {
                image: fermah_common::executable::Image::RemoteDocker((
                    fermah_common::resources::RemoteResource {
                        url: "http://localhost:8082/groth16_latest.tar.gz"
                            .parse()
                            .unwrap(),
                        hash: Blake3Hash(Hash::from_bytes([
                            50, 221, 218, 67, 63, 28, 144, 6, 179, 87, 107, 196, 204, 25, 85, 98,
                            129, 101, 39, 3, 247, 53, 4, 133, 3, 27, 254, 80, 82, 203, 249, 232,
                        ])),
                    },
                    "groth16:latest".to_string(),
                )),
                platform: None,
                in_mounts: vec![],
                result_extractor: Some(ResultExtractor::File("/output/state.bin".parse().unwrap())),
                injector: None,
                entrypoint: vec!["/bin/prove".into()],
                cmd: vec![],
                env_vars: Some(HashMap::from([(
                    "STATE_LOCATION".into(),
                    "/output/state.bin".into(),
                )])),
                network_enabled: false,
                privileged: false,
            },
            verifier: Executable {
                image: fermah_common::executable::Image::RemoteDocker((
                    fermah_common::resources::RemoteResource {
                        url: "http://localhost:8082/groth16_latest.tar.gz"
                            .parse()
                            .unwrap(),
                        hash: Blake3Hash(Hash::from_bytes([
                            50, 221, 218, 67, 63, 28, 144, 6, 179, 87, 107, 196, 204, 25, 85, 98,
                            129, 101, 39, 3, 247, 53, 4, 133, 3, 27, 254, 80, 82, 203, 249, 232,
                        ])),
                    },
                    "groth16:latest".to_string(),
                )),
                platform: None,
                in_mounts: vec![],
                result_extractor: Some(ResultExtractor::NegativeExitCode(58)),
                injector: Some(fermah_common::executable::Injector::File(
                    "/output/state.bin".into(),
                )),
                entrypoint: vec!["/bin/verify".into()],
                cmd: vec![],
                env_vars: Some(HashMap::from([(
                    "STATE_LOCATION".into(),
                    "/output/state.bin".into(),
                )])),
                network_enabled: false,
                privileged: false,
            },
            resource_requirement: ResourceRequirement {
                min_vram: None,
                min_ram: None,
                min_ssd: None,
                min_gpu: vec![],
                min_cpu_cores: Some(2),
            },
            nonce: 217,
            callback_url: None,
            deadline: None,
        };

        let secret_key = SigningKey::random(&mut StdRng::seed_from_u64(0));
        let signer = EcdsaSigner::from_key(secret_key);

        let p = ProofRequestParams {
            signed_payload: SignedData::new(pr, &signer).unwrap(),
            assigned: None,
            status: ProofStatus::Accepted,
            last_status_update: Utc::now(),
            payment: Payment::Nothing,
        };

        let s = serde_json::to_string_pretty(&p).unwrap();

        println!("{}", s);

        let rs: ProofRequestParams = serde_json::from_str(&s).unwrap();
        assert_eq!(p, rs);
        println!("{:?}", rs);

        let x = bincode::serialize(&p).unwrap();

        let x = bincode::deserialize::<ProofRequestParams>(&x).unwrap();

        assert_eq!(x, p)
    }
}
