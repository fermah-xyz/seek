use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{
    dsl::{insert_into, now, IntervalDsl},
    prelude::*,
    update,
};
use ethers::types::{Address, U256};
use fermah_common::{
    crypto::signer::{ecdsa::EcdsaSigner, SignedData},
    hash::blake3::Blake3Hash,
    operator::OperatorId,
    proof::{
        request::{ProofRequest, ProofRequestId},
        status::ProofStatus,
    },
};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use crate::{
    mm_operators::OperatorInfo,
    models::{self, EthAddress, EthU256, MmProofRequest},
    Database,
};

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

impl Database {
    pub(crate) fn now() -> NaiveDateTime {
        let now_ = Utc::now();
        let nanos = now_.timestamp_subsec_nanos();
        now_.naive_utc() - chrono::Duration::nanoseconds(nanos.into())
    }
    /// Returns operators that are not occupied by any tasks
    pub fn available_operators(&self) -> Result<Vec<OperatorInfo>> {
        use crate::schema::{mm_operators::dsl::*, mm_proof_requests::dsl::*};

        let mut conn = self
            .pool
            .get()
            .context("available_operators: failed to connect to the database")?;

        let occupied_operator_query = mm_proof_requests
            .filter(
                status
                    .eq(models::PrStatus::Assigned)
                    .or(status.eq(models::PrStatus::AcknowledgedAssignment)),
            )
            .filter(operator_id.is_not_null())
            .select(operator_id.assume_not_null());

        let operator_infos = mm_operators
            .filter(crate::schema::mm_operators::columns::id.ne_all(occupied_operator_query))
            .select(models::MmOperator::as_select())
            .load(&mut conn)
            .context("query available_operators failed")?;

        let operator_infos = operator_infos
            .into_iter()
            .map(OperatorInfo::from)
            .filter_map(|operator_info| {
                if !operator_info.is_online() {
                    None
                } else {
                    Some(operator_info)
                }
            })
            .collect();

        Ok(operator_infos)
    }

    #[cfg(test)]
    fn force_status(&self, proof_request_id: &ProofRequestId, status_: ProofStatus) -> Result<()> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("force_status: failed to connect to the database")?;

        update(mm_proof_requests.filter(id.eq(proof_request_id.as_32_bytes())))
            .set((
                last_status_update.eq(Self::now()),
                status.eq(models::PrStatus::from(status_.clone())),
            ))
            .execute(&mut conn)
            .context("query force_status failed")?;
        Ok(())
    }

    pub fn set_proof_request_status(
        &self,
        proof_request_id: &ProofRequestId,
        status_: ProofStatus,
    ) -> Result<()> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("set_proof_request_status: failed to connect to the database")?;

        let n = match status_.clone() {
            ProofStatus::Created => {
                warn!(
                    ?proof_request_id,
                    "denied setting proof request status to Created"
                );
                0
            }
            ProofStatus::Accepted | ProofStatus::Cancelled => {
                update(
                    mm_proof_requests.filter(
                        status
                            .eq(models::PrStatus::Created)
                            .and(id.eq(proof_request_id.as_32_bytes())),
                    ),
                )
                .set((
                    last_status_update.eq(Self::now()),
                    status.eq(models::PrStatus::from(status_.clone())),
                ))
                .execute(&mut conn)
                .context("query set_proof_request_status::Accepted | Cancelled failed")?
            }
            ProofStatus::Rejected(reason) => {
                update(
                    mm_proof_requests.filter(
                        status
                            .eq_any(vec![
                                models::PrStatus::Created,
                                models::PrStatus::AcknowledgedAssignment,
                                models::PrStatus::ProofBeingTested,
                            ])
                            .and(id.eq(proof_request_id.as_32_bytes())),
                    ),
                )
                .set((
                    last_status_update.eq(Self::now()),
                    status.eq(models::PrStatus::from(status_.clone())),
                    rejection_message.eq(reason),
                ))
                .execute(&mut conn)
                .context("query set_proof_request_status::Rejected failed")?
            }
            ProofStatus::Assigned(oid) => {
                let n = update(
                    mm_proof_requests.filter(
                        status
                            .eq(models::PrStatus::Accepted)
                            .and(id.eq(proof_request_id.as_32_bytes())),
                    ),
                )
                .set((
                    last_status_update.eq(Self::now()),
                    status.eq(models::PrStatus::from(status_.clone())),
                    operator_id.eq(EthAddress::from(oid)),
                ))
                .execute(&mut conn)
                .context("query set_proof_request_status::Assigned failed")?;

                if n == 0 {
                    warn!(
                        ?proof_request_id,
                        "failed to set proof request status to Assigned"
                    );
                }

                Self::set_last_assignment(&mut conn, oid)?
            }
            ProofStatus::AcknowledgedAssignment(oid) => {
                update(
                    mm_proof_requests.filter(
                        status
                            .eq(models::PrStatus::Assigned)
                            .and(id.eq(proof_request_id.as_32_bytes())),
                    ),
                )
                .set((
                    last_status_update.eq(Self::now()),
                    status.eq(models::PrStatus::from(status_.clone())),
                    operator_id.eq(EthAddress::from(oid)),
                ))
                .execute(&mut conn)
                .context("query set_proof_request_status::AcknowledgedAssignment failed")?
            }
            ProofStatus::ProofBeingTested(p) => {
                update(
                    mm_proof_requests.filter(
                        status
                            .eq(models::PrStatus::AcknowledgedAssignment)
                            .and(id.eq(proof_request_id.as_32_bytes())),
                    ),
                )
                .set((
                    last_status_update.eq(Self::now()),
                    status.eq(models::PrStatus::from(status_.clone())),
                    proof.eq(bincode::serialize(&p).unwrap()),
                    // operator_id must be null here
                ))
                .execute(&mut conn)
                .context("query set_proof_request_status::ProofBeingTested failed")?
            }
            ProofStatus::Proven(p) => {
                update(
                    mm_proof_requests.filter(
                        status
                            .eq(models::PrStatus::ProofBeingTested)
                            .and(id.eq(proof_request_id.as_32_bytes())),
                    ),
                )
                .set((
                    last_status_update.eq(Self::now()),
                    status.eq(models::PrStatus::from(status_.clone())),
                    proof.eq(bincode::serialize(&p).unwrap()),
                    // operator_id must be null here
                ))
                .execute(&mut conn)
                .context("query set_proof_request_status::Proven failed")?
            }
        };

        if n == 0 {
            warn!(?proof_request_id, status=?status_, "Proof request status not updated");
        }

        Ok(())
    }

    pub fn set_payment_status(
        &self,
        proof_request_id: &ProofRequestId,
        payment_status: Payment,
    ) -> Result<()> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("set_payment_status: failed to connect to the database")?;

        let n = match payment_status {
            Payment::Nothing => {
                update(mm_proof_requests.filter(id.eq(proof_request_id.as_32_bytes())))
                    .set((
                        last_status_update.eq(Self::now()),
                        payment.eq(models::PrPayment::from(payment_status)),
                        // Should set `amount` to NULL?
                    ))
                    .execute(&mut conn)
                    .context("query set_payment_status::Nothing failed")?
            }
            Payment::ToReserve(value)
            | Payment::Reserved(value)
            | Payment::ReadyToPay(value)
            | Payment::Paid(value)
            | Payment::Refund(value) => {
                update(mm_proof_requests.filter(id.eq(proof_request_id.as_32_bytes())))
                    .set((
                        last_status_update.eq(Self::now()),
                        payment.eq(models::PrPayment::from(payment_status)),
                        amount.eq(EthU256::from(value)),
                    ))
                    .execute(&mut conn)
                    .context("query set_payment_status::* failed")?
            }
        };
        if n == 0 {
            warn!(?proof_request_id, status=?payment_status, "Proof request payment status not updated");
        }
        Ok(())
    }

    pub fn set_payment_to_ready(&self, proof_request_id: &ProofRequestId) -> Result<()> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context(": failed to connect to the database")?;

        let n = update(
            mm_proof_requests.filter(
                id.eq(proof_request_id.as_32_bytes())
                    .and(payment.eq(models::PrPayment::Reserved)),
            ),
        )
        .set((
            last_status_update.eq(Self::now()),
            payment.eq(models::PrPayment::ReadyToPay),
        ))
        .execute(&mut conn)
        .context("query set_payment_to_ready failed")?;

        if n != 1 {
            let maybe_payments: Vec<(models::PrPayment, Option<EthU256>)> = mm_proof_requests
                .filter(id.eq(proof_request_id.as_32_bytes()))
                .select((payment, amount))
                .load(&mut conn)
                .with_context(|| {
                    format!("failed to query payment status for request id {proof_request_id:?}")
                })?;
            error!(
                ?proof_request_id,
                ?maybe_payments,
                "failed to set payment to ready"
            );
            bail!("failed to set payment to ready");
        }

        Ok(())
    }

    pub fn get_reserved_for_requester(&self, proof_requester: Address) -> Result<U256> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("get_reserved_for_requester: failed to connect to the database")?;

        let amounts: Vec<Option<EthU256>> = mm_proof_requests
            .filter(
                public_key
                    .eq(EthAddress::from(proof_requester))
                    .and(payment.eq(models::PrPayment::Reserved)),
            )
            .select(amount)
            .load(&mut conn)
            .context("query get_reserved_for_requester failed")?;

        Ok(amounts
            .into_iter()
            .filter_map(|a| a.map(U256::from))
            .fold(U256::zero(), |acc, e| acc + e))
    }

    pub fn try_create_proof_request(
        &self,
        proof_request: SignedData<ProofRequest, EcdsaSigner>,
    ) -> Result<Blake3Hash> {
        use crate::schema::mm_proof_requests::dsl::*;
        let proof_request_id = proof_request.hash;
        let mut conn = self
            .pool
            .get()
            .context("try_create_proof_request: failed to connect to the database")?;

        let n = insert_into(mm_proof_requests)
            .values((
                id.eq(proof_request_id.as_32_bytes()),
                last_status_update.eq(Self::now()),
                // Payment
                payment.eq(models::PrPayment::Nothing),
                // Payload
                hash.eq(proof_request.hash.as_32_bytes()),
                public_key.eq(EthAddress::from(proof_request.public_key)),
                payload.eq(bincode::serialize(&proof_request).unwrap()),
                signature.eq(bincode::serialize(&proof_request.signature).unwrap()),
                requester.eq(proof_request.payload.requester.map(EthAddress::from)),
                // Request status
                status.eq(models::PrStatus::Created),
            ))
            .on_conflict(id)
            .do_nothing()
            .execute(&mut conn)
            .context("query try_create_proof_request failed")?;

        if n != 1 {
            warn!(id=?proof_request_id, "failed to create proof request: {n} records already exist");
        }
        ensure!(n == 1, "failed to create proof request: already exists");

        Ok(proof_request_id)
    }

    const REASSIGNMENT_SECONDS: f64 = 10.0;
    //// note: We use SignedData<ProofRequest, EthSigner>, and not the PR itself, because particularly SignedData<ProofRequest, EthSigner> provides the `.id()`
    ////       method for PR
    //// todo: Ideally it should also include some metadata, such as timestamp of when we acknowledged the PR, so that we can
    ////       prioritize PRs, and also discard them if they
    ///// Proof requests that are ready for assignment. Note: requests, that were not Acknowledged for N seconds, are also returned for reassignment
    pub fn proof_requests_need_assignment(
        &self,
    ) -> Result<Vec<SignedData<ProofRequest, EcdsaSigner>>> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("proof_requests_need_assignment: failed to connect to the database")?;

        let maybe_proof_request_param: Vec<Vec<u8>> = mm_proof_requests
            .filter(status.eq(models::PrStatus::Accepted))
            .or_filter(
                status
                    .eq(models::PrStatus::Assigned)
                    .and(last_status_update.le(now - Self::REASSIGNMENT_SECONDS.seconds())),
            )
            .select(payload)
            .load(&mut conn)
            .context("query proof_requests_need_assignment failed")?;

        let proof_requests = maybe_proof_request_param
            .into_iter()
            .map(|p| bincode::deserialize(&p).unwrap())
            .collect();

        Ok(proof_requests)
    }

    pub fn set_proof_requests_paid(&self, proof_request_ids: &Vec<ProofRequestId>) -> Result<()> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("set_proof_requests_paid: failed to connect to the database")?;

        let proof_requests = proof_request_ids
            .iter()
            .map(|pr| pr.as_32_bytes())
            .collect::<Vec<_>>();

        let n = update(mm_proof_requests)
            .filter(id.eq_any(proof_requests))
            .filter(payment.eq(models::PrPayment::ReadyToPay))
            .set((payment.eq(models::PrPayment::Paid),))
            .execute(&mut conn)
            .context("query set_proof_requests_paid failed")?;

        if n == 0 {
            warn!(?proof_request_ids, "no proof request were set to Paid");
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
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context(": failed to connect to the database")?;

        let proof_requests: Vec<(
            Option<EthAddress>,
            Option<EthAddress>,
            Option<EthU256>,
            Vec<u8>,
        )> = mm_proof_requests
            .filter(payment.eq(models::PrPayment::ReadyToPay))
            .filter(assigned.is_not_null())
            .filter(requester.is_not_null())
            .filter(amount.is_not_null()) // Note that amount is `Some(fund)` but fund may be 0.
            .select((assigned, requester, amount, id))
            .load(&mut conn)
            .context("query get_ready_to_pay_proof_requests_for_many failed")?;

        // let proof_requests: Vec<(OperatorId, Address, U256, Blake3Hash)> = proof_requests
        let proof_requests: Vec<(OperatorId, Address, U256, Blake3Hash)> = proof_requests
            .into_iter()
            .map(|(operator_id_, requester_, amount_, pr_id)| {
                (
                    OperatorId::from(operator_id_.unwrap()),
                    Address::from(requester_.unwrap()),
                    U256::from(amount_.unwrap()),
                    Blake3Hash::from(pr_id),
                )
            })
            .collect();

        let mut payments: HashMap<OperatorId, HashMap<Address, U256>> = HashMap::new();
        let mut to_be_paid = vec![];

        for (prover, requester_, amount_, pr_id) in proof_requests.into_iter() {
            if let Some(p) = payments.get_mut(&prover) {
                if let Some(to_pay) = p.get_mut(&requester_) {
                    if to_pay.checked_add(amount_).is_none() {
                        // todo: finish it
                        bail!("Overflow occured")
                    }
                } else {
                    p.insert(requester_, amount_);
                }
            } else {
                payments.insert(prover, HashMap::from([(requester_, amount_)]));
            }
            to_be_paid.push(pr_id);
        }
        Ok((payments, to_be_paid))
    }

    #[allow(clippy::type_complexity)]
    pub fn get_ready_to_pay_proof_requests(
        &self,
        operator_id_: &OperatorId,
    ) -> Result<(HashMap<Address, U256>, Vec<ProofRequestId>)> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("get_ready_to_pay_proof_requests: failed to connect to the database")?;

        let proof_requests: Vec<(Option<EthAddress>, Option<EthU256>, Vec<u8>)> = mm_proof_requests
            .filter(payment.eq(models::PrPayment::ReadyToPay))
            .filter(assigned.eq(EthAddress::from(*operator_id_)))
            .filter(requester.is_not_null())
            .filter(amount.is_not_null()) // Note that amount is `Some(fund)` but fund may be 0.
            .select((requester, amount, id))
            .load(&mut conn)
            .context("query get_ready_to_pay_proof_requests failed")?;

        let proof_requests: Vec<(Address, U256, Blake3Hash)> = proof_requests
            .into_iter()
            .map(|(requester_, amount_, pr_id)| {
                (
                    Address::from(requester_.unwrap()),
                    U256::from(amount_.unwrap()),
                    Blake3Hash::from(pr_id),
                )
            })
            .collect();

        let mut payments: HashMap<Address, U256> = HashMap::new();
        let mut to_be_paid = vec![];

        for (requester_, amount_, pr_id) in proof_requests.into_iter() {
            if let Some(to_pay) = payments.get_mut(&requester_) {
                if to_pay.checked_add(amount_).is_none() {
                    // todo: finish it
                    bail!("Overflow occured")
                }
            } else {
                payments.insert(requester_, amount_);
            }
            to_be_paid.push(pr_id);
        }
        Ok((payments, to_be_paid))
    }

    // Closes existing unassigned PRs and returns amount of money which is already reserved for payment, to deduct it later.
    pub fn non_refundable_amount(&self, proof_requester: &Address) -> Result<U256> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("non_refundable_amount: failed to connect to the database")?;

        // Query that tells if there are any money that should be witheld from returning to the proof requester
        // note: this doesn't take into account those PRs that were processed, but due to unsatisfactory results PRer's funds could be returned
        // note: this generally doesn't take into account another field `status`, with which, Params should have status and payment merged somehow
        let non_refundable: Vec<Option<EthU256>> = mm_proof_requests
            .filter(public_key.eq(EthAddress::from(*proof_requester)))
            .filter(
                payment
                    .eq(models::PrPayment::ReadyToPay)
                    .or(payment.eq(models::PrPayment::Reserved)),
            )
            .filter(amount.is_not_null())
            .select(amount)
            .load(&mut conn)
            .context("query non_refundable_amount failed")?;

        let non_refundable = non_refundable
            .into_iter()
            .fold(U256::zero(), |acc, amount_| {
                if let Some(acc) = acc.checked_add(amount_.unwrap_or_default().into()) {
                    acc
                } else {
                    error!("Failed to reserve for not refundable");
                    U256::max_value()
                }
            });

        Ok(non_refundable)
    }

    pub fn get_proof_request(
        &self,
        proof_request_id: &ProofRequestId,
    ) -> Result<Option<ProofRequestParams>> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("get_proof_request: failed to connect to the database")?;

        let maybe_proof_request_param = mm_proof_requests
            .filter(id.eq(proof_request_id.as_32_bytes()))
            .select(MmProofRequest::as_select())
            .first(&mut conn)
            .map(ProofRequestParams::from)
            .optional()
            .context("query get_proof_request failed")?;

        Ok(maybe_proof_request_param)
    }

    #[cfg(test)]
    pub fn get_full_proof_request(
        &self,
        proof_request_id: &ProofRequestId,
    ) -> Result<Option<MmProofRequest>> {
        use crate::schema::mm_proof_requests::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("get_full_proof_request: failed to connect to the database")?;

        let maybe_proof_request_param = mm_proof_requests
            .filter(id.eq(proof_request_id.as_32_bytes()))
            .select(MmProofRequest::as_select())
            .first(&mut conn)
            .optional()
            .context("query get_full_proof_request failed")?;

        Ok(maybe_proof_request_param)
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use fermah_common::proof::Proof;

    use super::*;
    use crate::database_test::TestContext;

    const PROOF_REQUEST_JSON: &str = r##"{"hash":"0x99e6070bde0937991360bdc960ef7f683cd8b3d6514f30ac4f2b04283c76c803","payload":{"requester":"0x70997970c51812dc3a010c7d01b50e0d17dc79c8","prover":{"image":{"remoteDocker":[{"url":"http://localhost:3000/images/groth16_latest.tar.gz","hash":"0x2a7504ffa9ca644ffbd70d76d3ad30795878a2d3efcc37416368e01da44baf39"},"groth16:latest"]},"platform":null,"inMounts":[],"resultExtractor":{"file":"/output/state.bin"},"injector":null,"entrypoint":["/bin/prove"],"cmd":[],"envVars":{"STATE_LOCATION":"/output/state.bin"},"networkEnabled":false,"privileged":false,"dockerAccess":false},"verifier":{"image":{"remoteDocker":[{"url":"http://localhost:3000/images/groth16_latest.tar.gz","hash":"0x2a7504ffa9ca644ffbd70d76d3ad30795878a2d3efcc37416368e01da44baf39"},"groth16:latest"]},"platform":null,"inMounts":[],"resultExtractor":{"negativeExitCode":58},"injector":{"file":"/output/state.bin"},"entrypoint":["/bin/verify"],"cmd":[],"envVars":{"STATE_LOCATION":"/output/state.bin"},"networkEnabled":false,"privileged":false,"dockerAccess":false},"resourceRequirement":{"minVram":null,"minRam":null,"minSsd":null,"minGpu":[],"minCpuCores":2},"callbackUrl":null,"deadline":null,"nonce":217},"publicKey":"0x70997970c51812dc3a010c7d01b50e0d17dc79c8","signature":{"r":"0xf166dc59d3b6fb2d532c106255c611cfb351bd9d018aff843df4736981e01fd1","s":"0xfcf3ae33229729552c47e35ea2e9ae0bd233762c2365a8f1bedad0abbb8cfad","v":27}}"##;

    #[test]
    fn create_pr() {
        let _ctx = TestContext::new("postgres://postgres:postgres@127.0.0.1", "create_pr");

        let db = Database::connect_to_database("postgres://postgres:postgres@127.0.0.1/create_pr")
            .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let proof_request_id = proof_request.hash;

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());

        let maybe_pr = db.get_proof_request(&proof_request_id);

        assert!(matches!(maybe_pr, Ok(Some(_))));
        let pr = maybe_pr.unwrap().unwrap();

        assert_eq!(pr.signed_payload, proof_request);
        assert_eq!(pr.payment, Payment::Nothing);
        assert_eq!(pr.status, ProofStatus::Created);

        let full_pr = db
            .get_full_proof_request(&proof_request_id)
            .unwrap()
            .unwrap();
        assert!(
            matches!((full_pr.requester, proof_request.payload.requester), (Some(got), Some(expected)) if Address::from(got) == expected)
        );
        assert_eq!(Blake3Hash::from(full_pr.hash), proof_request_id);
        assert!(db.try_create_proof_request(proof_request).is_err());
    }

    #[test]
    fn check_non_refundable_amount() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_non_refundable_amount",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_non_refundable_amount",
        )
        .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let proof_request_id = proof_request.hash;
        let proof_requester = proof_request.payload.requester.unwrap();
        let amount = U256::from_dec_str("54321").unwrap();

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());
        for payment_status in vec![
            Payment::ToReserve(amount),
            Payment::Reserved(amount * 2),
            Payment::ReadyToPay(amount * 3),
            Payment::Paid(amount * 4),
            Payment::Refund(amount * 5),
            Payment::Nothing,
        ] {
            assert!(db
                .set_payment_status(&proof_request_id, payment_status)
                .is_ok());

            let maybe_non_refundable = db.non_refundable_amount(&proof_requester);
            assert!(maybe_non_refundable.is_ok());
            let non_refundable = maybe_non_refundable.unwrap();

            match payment_status {
                Payment::ReadyToPay(amount) | Payment::Reserved(amount) => {
                    assert_eq!(non_refundable, amount)
                }
                _ => assert_eq!(non_refundable, U256::zero()),
            }
        }
    }

    #[test]
    fn update_pr_payment_status() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "update_pr_payment_status",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/update_pr_payment_status",
        )
        .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let proof_request_id = proof_request.hash;

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());
        let amount = U256::from_dec_str("54321").unwrap();

        for payment_status in vec![
            Payment::ToReserve(amount),
            Payment::Reserved(amount * 2),
            Payment::ReadyToPay(amount * 3),
            Payment::Paid(amount * 4),
            Payment::Refund(amount * 5),
            Payment::Nothing,
        ] {
            assert!(db
                .set_payment_status(&proof_request_id, payment_status)
                .is_ok());

            let maybe_pr = db.get_proof_request(&proof_request_id);

            assert!(matches!(maybe_pr, Ok(Some(_))));
            let pr = maybe_pr.unwrap().unwrap();
            assert_eq!(pr.payment, payment_status);
        }
    }

    #[test]
    fn check_set_payment_ready() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_set_payment_ready",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_set_payment_ready",
        )
        .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let proof_request_id = proof_request.hash;

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());
        let amount = U256::from_dec_str("54321").unwrap();

        for payment_status in vec![
            Payment::ToReserve(amount),
            Payment::Reserved(amount * 2),
            Payment::ReadyToPay(amount * 3),
            Payment::Paid(amount * 4),
            Payment::Refund(amount * 5),
            Payment::Nothing,
        ] {
            assert!(db
                .set_payment_status(&proof_request_id, payment_status)
                .is_ok());

            if let Payment::Reserved(amount) = payment_status {
                assert!(db.set_payment_to_ready(&proof_request_id).is_ok());
                let maybe_pr = db.get_proof_request(&proof_request_id);

                assert!(matches!(maybe_pr, Ok(Some(_))));
                let pr = maybe_pr.unwrap().unwrap();
                assert_eq!(pr.payment, Payment::ReadyToPay(amount));
            } else {
                assert!(
                    db.set_payment_to_ready(&proof_request_id).is_err(),
                    "Payment shouldn't be set to ready when payment status is {payment_status:?}"
                );
            }
        }
    }

    fn check_status(
        db: &Database,
        proof_request_id: &Blake3Hash,
        status: ProofStatus,
        expected: ProofStatus,
        test_name: &str,
    ) {
        let initial_status = db
            .get_proof_request(proof_request_id)
            .unwrap()
            .unwrap()
            .status;

        assert!(db
            .set_proof_request_status(proof_request_id, status.clone())
            .is_ok());

        let pr = db.get_proof_request(proof_request_id).unwrap().unwrap();
        assert_eq!(
            pr.status, expected,
            "Setting status to {status}, expecting: {expected} - test {test_name}"
        );

        // Reset status
        assert!(db.force_status(proof_request_id, initial_status).is_ok())
    }

    #[test]
    fn update_pr_status() {
        let _ctx = TestContext::new("postgres://postgres:postgres@127.0.0.1", "update_pr_status");

        let ps_created = ProofStatus::Created;
        let ps_accepted = ProofStatus::Accepted;
        let ps_rejected = ProofStatus::Rejected("sorry".into());
        let ps_assigned = ProofStatus::Assigned(Address::from_low_u64_be(123).into());
        let ps_ack_assignment =
            ProofStatus::AcknowledgedAssignment(Address::from_low_u64_be(123).into());
        let ps_being_tested = ProofStatus::ProofBeingTested(Proof {
            proof: vec![0, 1, 2, 4, 5, 0],
            prover: Address::random().into(),
        });
        let ps_proven = ProofStatus::Proven(Proof {
            proof: vec![0, 9, 6, 0],
            prover: Address::random().into(),
        });

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/update_pr_status",
        )
        .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let pr_id = proof_request.hash;

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());
        // Test state machine

        // CREATED

        for (status, expected) in vec![
            (ps_created.clone(), ps_created.clone()),
            (ps_accepted.clone(), ps_accepted.clone()),
            (ps_rejected.clone(), ps_rejected.clone()),
            (ps_assigned.clone(), ps_created.clone()),
            (ps_ack_assignment.clone(), ps_created.clone()),
            (ps_being_tested.clone(), ps_created.clone()),
            (ps_proven.clone(), ps_created.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "CREATED");
        }

        // ACCEPTED

        assert!(db.force_status(&pr_id, ps_accepted.clone()).is_ok());

        for (status, expected) in vec![
            (ps_created.clone(), ps_accepted.clone()),
            (ps_accepted.clone(), ps_accepted.clone()),
            (ps_rejected.clone(), ps_accepted.clone()),
            (ps_assigned.clone(), ps_assigned.clone()),
            (ps_ack_assignment.clone(), ps_accepted.clone()),
            (ps_being_tested.clone(), ps_accepted.clone()),
            (ps_proven.clone(), ps_accepted.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "ACCEPTED");
        }

        // ASSIGNED

        assert!(db.force_status(&pr_id, ps_assigned.clone()).is_ok());

        for (status, expected) in vec![
            (ps_created.clone(), ps_assigned.clone()),
            (ps_accepted.clone(), ps_assigned.clone()),
            (ps_rejected.clone(), ps_assigned.clone()),
            (ps_assigned.clone(), ps_assigned.clone()),
            (ps_ack_assignment.clone(), ps_ack_assignment.clone()),
            (ps_being_tested.clone(), ps_assigned.clone()),
            (ps_proven.clone(), ps_assigned.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "ASSIGNED");
        }
        // ACCEPTED

        assert!(db.force_status(&pr_id, ps_accepted.clone()).is_ok());
        for (status, expected) in vec![
            (ps_created.clone(), ps_accepted.clone()),
            (ps_accepted.clone(), ps_accepted.clone()),
            (ps_rejected.clone(), ps_accepted.clone()),
            (ps_assigned.clone(), ps_assigned.clone()),
            (ps_ack_assignment.clone(), ps_accepted.clone()),
            (ps_being_tested.clone(), ps_accepted.clone()),
            (ps_proven.clone(), ps_accepted.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "ACCEPTED");
        }

        // ASSIGNED

        assert!(db.force_status(&pr_id, ps_assigned.clone()).is_ok());
        for (status, expected) in vec![
            (ps_created.clone(), ps_assigned.clone()),
            (ps_accepted.clone(), ps_assigned.clone()),
            (ps_rejected.clone(), ps_assigned.clone()),
            (ps_assigned.clone(), ps_assigned.clone()),
            (ps_ack_assignment.clone(), ps_ack_assignment.clone()),
            (ps_being_tested.clone(), ps_assigned.clone()),
            (ps_proven.clone(), ps_assigned.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "ASSIGNED");
        }

        // ACKNOWLEDGE_ASSIGNMENT

        assert!(db.force_status(&pr_id, ps_ack_assignment.clone()).is_ok());
        for (status, expected) in vec![
            (ps_created.clone(), ps_ack_assignment.clone()),
            (ps_accepted.clone(), ps_ack_assignment.clone()),
            (ps_rejected.clone(), ps_rejected.clone()),
            (ps_assigned.clone(), ps_ack_assignment.clone()),
            (ps_ack_assignment.clone(), ps_ack_assignment.clone()),
            (ps_being_tested.clone(), ps_being_tested.clone()),
            (ps_proven.clone(), ps_ack_assignment.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "ACKNOWLEDGE_ASSIGNMENT");
        }

        // PROOF_BEING_TESTED

        assert!(db.force_status(&pr_id, ps_being_tested.clone()).is_ok());
        for (status, expected) in vec![
            (ps_created.clone(), ps_being_tested.clone()),
            (ps_accepted.clone(), ps_being_tested.clone()),
            (ps_rejected.clone(), ps_rejected.clone()),
            (ps_assigned.clone(), ps_being_tested.clone()),
            (ps_ack_assignment.clone(), ps_being_tested.clone()),
            (ps_being_tested.clone(), ps_being_tested.clone()),
            (ps_proven.clone(), ps_proven.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "PROOF_BEING_TESTED");
        }

        // REJECTED

        assert!(db.force_status(&pr_id, ps_rejected.clone()).is_ok());
        for (status, expected) in vec![
            (ps_created.clone(), ps_rejected.clone()),
            (ps_accepted.clone(), ps_rejected.clone()),
            (ps_rejected.clone(), ps_rejected.clone()),
            (ps_assigned.clone(), ps_rejected.clone()),
            (ps_ack_assignment.clone(), ps_rejected.clone()),
            (ps_being_tested.clone(), ps_rejected.clone()),
            (ps_proven.clone(), ps_rejected.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "REJECTED");
        }

        // PROVEN

        assert!(db.force_status(&pr_id, ps_proven.clone()).is_ok());
        for (status, expected) in vec![
            (ps_created.clone(), ps_proven.clone()),
            (ps_accepted.clone(), ps_proven.clone()),
            (ps_rejected.clone(), ps_proven.clone()),
            (ps_assigned.clone(), ps_proven.clone()),
            (ps_ack_assignment.clone(), ps_proven.clone()),
            (ps_being_tested.clone(), ps_proven.clone()),
            (ps_proven.clone(), ps_proven.clone()),
        ] {
            self::check_status(&db, &pr_id, status, expected, "PROVEN");
        }
    }

    #[test]
    fn check_set_pr_paid() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_set_pr_paid",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_set_pr_paid",
        )
        .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let proof_request_id = proof_request.hash;
        let proof_request_ids = vec![proof_request.hash];

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());
        let amount = U256::from_dec_str("54321").unwrap();
        for payment_status in vec![
            Payment::ToReserve(amount),
            Payment::Reserved(amount * 2),
            Payment::ReadyToPay(amount * 3),
            Payment::Paid(amount * 4),
            Payment::Refund(amount * 5),
            Payment::Nothing,
        ] {
            assert!(db
                .set_payment_status(&proof_request_id, payment_status)
                .is_ok());

            assert!(db.set_proof_requests_paid(&proof_request_ids).is_ok());
            let pr = db.get_proof_request(&proof_request_id).unwrap().unwrap();

            if let Payment::ReadyToPay(amount) = payment_status {
                assert_eq!(pr.payment, Payment::Paid(amount));
            } else {
                assert_eq!(pr.payment, payment_status);
            }
        }
    }

    #[tokio::test]
    async fn check_need_assignement() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_need_assignement",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_need_assignement",
        )
        .unwrap();
        let proof_request: SignedData<ProofRequest, EcdsaSigner> =
            serde_json::from_str(PROOF_REQUEST_JSON).unwrap();

        let proof_request_id = proof_request.hash;

        assert!(db.try_create_proof_request(proof_request.clone()).is_ok());

        // `proof_requests_need_assignment` returns an empty list for statuses different from assigned or accepted
        for status in vec![
            ProofStatus::Created,
            ProofStatus::Cancelled,
            ProofStatus::Rejected("Sorry".into()),
            ProofStatus::AcknowledgedAssignment(Address::random().into()),
            ProofStatus::ProofBeingTested(Proof {
                proof: vec![0, 1, 2, 4, 5, 0],
                prover: Address::random().into(),
            }),
            ProofStatus::Proven(Proof {
                proof: vec![0, 9, 6, 0],
                prover: Address::random().into(),
            }),
        ] {
            assert!(db.force_status(&proof_request_id, status.clone()).is_ok());
            assert!(matches!(db.proof_requests_need_assignment(), Ok(prs) if prs.is_empty()));
        }

        // Check Accepted
        assert!(db
            .force_status(&proof_request_id, ProofStatus::Accepted)
            .is_ok());
        assert!(
            matches!(db.proof_requests_need_assignment(), Ok(prs) if prs== vec![proof_request.clone()])
        );

        // Check Assigned
        assert!(db
            .force_status(
                &proof_request_id,
                ProofStatus::Assigned(Address::random().into())
            )
            .is_ok());
        assert!(matches!(db.proof_requests_need_assignment(), Ok(prs) if prs.is_empty()));

        tokio::time::sleep(Duration::from_secs_f64(Database::REASSIGNMENT_SECONDS / 2.)).await;
        assert!(matches!(db.proof_requests_need_assignment(), Ok(prs) if prs.is_empty()));

        tokio::time::sleep(Duration::from_secs_f64(
            Database::REASSIGNMENT_SECONDS / 2. + 1.,
        ))
        .await;
        assert!(
            matches!(db.proof_requests_need_assignment(), Ok(prs) if prs== vec![proof_request.clone()])
        );
    }
}
