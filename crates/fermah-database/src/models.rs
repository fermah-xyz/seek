use std::{io::Write, str::FromStr};

use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::{Pg, PgValue},
    prelude::*,
    serialize::{IsNull, Output, ToSql},
    sql_types::{Bytea, Numeric},
};
use ethers::types::{Address, U256};
use fermah_common::{operator::OperatorId, proof::status::ProofStatus};
use tracing::{error, warn};

use crate::{
    mm_operators::OperatorInfo,
    mm_proof_requests::{Payment, ProofRequestParams},
};

#[derive(Debug, Clone, Copy, FromSqlRow, AsExpression, Default)]
#[diesel(sql_type = Bytea)]
pub struct EthAddress(pub Address);

impl From<Address> for EthAddress {
    fn from(value: Address) -> Self {
        Self(value)
    }
}

impl From<EthAddress> for Address {
    fn from(value: EthAddress) -> Self {
        value.0
    }
}

impl From<OperatorId> for EthAddress {
    fn from(value: OperatorId) -> Self {
        Self(value.0)
    }
}

impl From<EthAddress> for OperatorId {
    fn from(value: EthAddress) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, Copy, FromSqlRow, AsExpression, Default)]
#[diesel(sql_type = Numeric)]
pub struct EthU256(U256);
impl From<U256> for EthU256 {
    fn from(value: U256) -> Self {
        Self(value)
    }
}

impl From<EthU256> for U256 {
    fn from(value: EthU256) -> Self {
        value.0
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::avs_proof_requesters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ProofRequester {
    pub id: EthAddress,
    #[diesel(serialize_as = EthU256, deserialize_as = EthU256)]
    pub deposit: U256,
}

impl ToSql<Bytea, Pg> for EthAddress {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        ToSql::<Bytea, Pg>::to_sql(self.0.as_bytes(), &mut out.reborrow())
    }
}

impl FromSql<Bytea, Pg> for EthAddress {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let bytes: Vec<u8> = FromSql::<Bytea, Pg>::from_sql(bytes)?;
        Ok(Self(Address::from_slice(&bytes)))
    }
}

impl FromSql<Numeric, Pg> for EthU256 {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let n = <bigdecimal::BigDecimal as FromSql<Numeric, Pg>>::from_sql(bytes)?.to_string();
        Ok(Self(U256::from_dec_str(&n).unwrap()))
    }
}

impl ToSql<Numeric, Pg> for EthU256 {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        let n = BigDecimal::from_str(&self.0.to_string()).unwrap();
        <bigdecimal::BigDecimal as ToSql<Numeric, Pg>>::to_sql(&n, &mut out.reborrow())
    }
}

#[derive(Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::PrStatus)]
pub enum PrStatus {
    Created,
    Accepted,
    Cancelled,
    Rejected,
    Assigned,
    AcknowledgedAssignment,
    ProofBeingTested,
    Proven,
}

impl From<ProofStatus> for PrStatus {
    fn from(value: ProofStatus) -> Self {
        match value {
            ProofStatus::Created => PrStatus::Created,
            ProofStatus::Accepted => PrStatus::Accepted,
            ProofStatus::Cancelled => PrStatus::Cancelled,
            ProofStatus::Rejected(_) => PrStatus::Rejected,
            ProofStatus::Assigned(_) => PrStatus::Assigned,
            ProofStatus::AcknowledgedAssignment(_) => PrStatus::AcknowledgedAssignment,
            ProofStatus::ProofBeingTested(_) => PrStatus::ProofBeingTested,
            ProofStatus::Proven(_) => PrStatus::Proven,
        }
    }
}

impl ToSql<crate::schema::sql_types::PrStatus, Pg> for PrStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            PrStatus::Created => out.write_all(b"Created")?,
            PrStatus::Accepted => out.write_all(b"Accepted")?,
            PrStatus::Cancelled => out.write_all(b"Cancelled")?,
            PrStatus::Rejected => out.write_all(b"Rejected")?,
            PrStatus::Assigned => out.write_all(b"Assigned")?,
            PrStatus::AcknowledgedAssignment => out.write_all(b"AcknowledgedAssignment")?,
            PrStatus::ProofBeingTested => out.write_all(b"ProofBeingTested")?,
            PrStatus::Proven => out.write_all(b"Proven")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::PrStatus, Pg> for PrStatus {
    fn from_sql(bytes: PgValue) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Created" => Ok(PrStatus::Created),
            b"Accepted" => Ok(PrStatus::Accepted),
            b"Cancelled" => Ok(PrStatus::Cancelled),
            b"Rejected" => Ok(PrStatus::Rejected),
            b"Assigned" => Ok(PrStatus::Assigned),
            b"AcknowledgedAssignment" => Ok(PrStatus::AcknowledgedAssignment),
            b"ProofBeingTested" => Ok(PrStatus::ProofBeingTested),
            b"Proven" => Ok(PrStatus::Proven),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schema::sql_types::PrPayment)]
pub enum PrPayment {
    Nothing,
    ToReserve,
    Reserved,
    ReadyToPay,
    Paid,
    Refund,
}

impl From<Payment> for PrPayment {
    fn from(value: Payment) -> Self {
        match value {
            Payment::Nothing => PrPayment::Nothing,
            Payment::ToReserve(_) => PrPayment::ToReserve,
            Payment::Reserved(_) => PrPayment::Reserved,
            Payment::ReadyToPay(_) => PrPayment::ReadyToPay,
            Payment::Paid(_) => PrPayment::Paid,
            Payment::Refund(_) => PrPayment::Refund,
        }
    }
}

impl ToSql<crate::schema::sql_types::PrPayment, Pg> for PrPayment {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            PrPayment::Nothing => out.write_all(b"Nothing")?,
            PrPayment::ToReserve => out.write_all(b"ToReserve")?,
            PrPayment::Reserved => out.write_all(b"Reserved")?,
            PrPayment::ReadyToPay => out.write_all(b"ReadyToPay")?,
            PrPayment::Paid => out.write_all(b"Paid")?,
            PrPayment::Refund => out.write_all(b"Refund")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<crate::schema::sql_types::PrPayment, Pg> for PrPayment {
    fn from_sql(bytes: PgValue) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Nothing" => Ok(PrPayment::Nothing),
            b"ToReserve" => Ok(PrPayment::ToReserve),
            b"Reserved" => Ok(PrPayment::Reserved),
            b"ReadyToPay" => Ok(PrPayment::ReadyToPay),
            b"Paid" => Ok(PrPayment::Paid),
            b"Refund" => Ok(PrPayment::Refund),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::mm_proof_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MmProofRequestAmount {
    pub amount: Option<EthU256>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::mm_proof_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MmProofRequest {
    pub assigned: Option<EthAddress>,
    pub last_status_update: NaiveDateTime,
    pub payment: PrPayment,
    pub amount: Option<EthU256>,
    pub hash: Vec<u8>,
    pub public_key: Vec<u8>,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
    pub requester: Option<EthAddress>,
    pub status: PrStatus,
    pub rejection_message: Option<String>,
    pub operator_id: Option<EthAddress>,
    pub proof: Option<Vec<u8>>,
}

impl From<MmProofRequest> for ProofRequestParams {
    fn from(value: MmProofRequest) -> Self {
        let status = match value.status {
            PrStatus::Created => ProofStatus::Created,
            PrStatus::Accepted => ProofStatus::Accepted,
            PrStatus::Cancelled => ProofStatus::Cancelled,
            PrStatus::Rejected => {
                if value.rejection_message.is_none() {
                    warn!("empty rejection_message");
                }
                ProofStatus::Rejected(value.rejection_message.unwrap_or_default())
            }
            PrStatus::Assigned => {
                if value.operator_id.is_none() {
                    warn!("empty operator_id");
                }
                ProofStatus::Assigned(value.operator_id.unwrap_or_default().into())
            }
            PrStatus::AcknowledgedAssignment => {
                if value.operator_id.is_none() {
                    warn!("empty operator_id");
                }
                ProofStatus::AcknowledgedAssignment(value.operator_id.unwrap_or_default().into())
            }
            PrStatus::ProofBeingTested => {
                if value.proof.is_none() {
                    error!("empty proof");
                }
                ProofStatus::ProofBeingTested(
                    bincode::deserialize(&value.proof.unwrap_or_default()).unwrap(),
                )
            }
            PrStatus::Proven => {
                if value.proof.is_none() {
                    error!("empty proof");
                }
                ProofStatus::Proven(bincode::deserialize(&value.proof.unwrap_or_default()).unwrap())
            }
        };

        let payment = match value.payment {
            PrPayment::Nothing => Payment::Nothing,
            PrPayment::ToReserve => {
                if value.amount.is_none() {
                    warn!("empty amount")
                }
                Payment::ToReserve(value.amount.unwrap_or_default().into())
            }
            PrPayment::Reserved => {
                if value.amount.is_none() {
                    warn!("empty amount")
                }
                Payment::Reserved(value.amount.unwrap_or_default().into())
            }
            PrPayment::ReadyToPay => {
                if value.amount.is_none() {
                    warn!("empty amount")
                }
                Payment::ReadyToPay(value.amount.unwrap_or_default().into())
            }
            PrPayment::Paid => {
                if value.amount.is_none() {
                    warn!("empty amount")
                }
                Payment::Paid(value.amount.unwrap_or_default().into())
            }
            PrPayment::Refund => {
                if value.amount.is_none() {
                    warn!("empty amount")
                }
                Payment::Refund(value.amount.unwrap_or_default().into())
            }
        };

        Self {
            signed_payload: bincode::deserialize(&value.payload).unwrap(),
            assigned: value.assigned.map(|oid| oid.into()),
            status,
            last_status_update: value.last_status_update.and_utc(),
            payment,
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::mm_operators)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MmOperator {
    pub id: EthAddress,
    pub last_interaction: NaiveDateTime,
    pub last_assignment: NaiveDateTime,
    pub resource: Vec<u8>,
    pub reputation: i64,
    pub online: bool,
}

impl From<MmOperator> for OperatorInfo {
    fn from(value: MmOperator) -> Self {
        Self {
            operator_id: value.id.into(),
            resource: bincode::deserialize(&value.resource).unwrap(),
            reputation: value.reputation,
            last_interaction: value.last_interaction.and_utc(),
            online: value.online,
            last_assignment: value.last_assignment.and_utc(),
        }
    }
}
