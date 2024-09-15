use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::{delete, insert_into},
    prelude::*,
    update,
};
use fermah_common::{operator::OperatorId, resource::Resource};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    models::{EthAddress, MmOperator},
    schema::mm_operators::dsl::*,
    Database,
    DbConnection,
};
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
    /// Last time a request was assigned to the operator
    pub last_assignment: DateTime<Utc>,
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

impl Database {
    pub fn register_operator_from_p2p(
        &self,
        operator_id: OperatorId,
        resource_: Resource,
    ) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("register_operator_from_p2p: failed to connect to the database")?;

        let n = insert_into(mm_operators)
            .values((
                id.eq(EthAddress::from(operator_id)),
                last_interaction.eq(Self::now()),
                resource.eq(bincode::serialize(&resource_).unwrap()),
                online.eq(true),
            ))
            .on_conflict(id)
            .do_nothing()
            .execute(&mut conn)
            .context("query register_operator_from_p2p failed")?;

        if n != 1 {
            info!(
                ?operator_id,
                "operator is registering again from the p2p network"
            )
        }

        Ok(())
    }

    pub fn unregister_operator_from_p2p(&self, operator_id: &OperatorId) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("unregister_operator_from_p2p: failed to connect to the database")?;
        let n = delete(mm_operators)
            .filter(id.eq(EthAddress::from(*operator_id)))
            .execute(&mut conn)
            .context("query unregister_operator_from_p2p failed")?;

        if n != 1 {
            info!(?operator_id, "trying to unregister an unknown operator")
        }

        Ok(())
    }

    pub fn get_operator(&self, operator_id: &OperatorId) -> Result<Option<OperatorInfo>> {
        let mut conn = self
            .pool
            .get()
            .context("get_operator: failed to connect to the database")?;

        let maybe_operator_info = mm_operators
            .filter(id.eq(EthAddress::from(*operator_id)))
            .select(MmOperator::as_select())
            .first(&mut conn)
            .map(OperatorInfo::from)
            .optional()
            .context("query get_operator failed")?;

        Ok(maybe_operator_info)
    }

    pub fn update_last_interaction(&self, operator_id: &OperatorId) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("update_last_interaction: failed to connect to the database")?;

        let n = update(mm_operators.filter(id.eq(EthAddress::from(*operator_id))))
            .set(last_interaction.eq(Self::now()))
            .execute(&mut conn)
            .context("query update_last_interaction failed")?;

        if n != 1 {
            warn!(
                ?operator_id,
                "Try to update last interaction for unknown operator"
            );
        }

        Ok(())
    }

    pub(crate) fn set_last_assignment(conn: &mut DbConnection, oid: OperatorId) -> Result<usize> {
        update(mm_operators.filter(id.eq(EthAddress::from(oid))))
            .set(last_assignment.eq(Self::now()))
            .execute(conn)
            .context("query set_last_assignment failed")
    }

    ///// Returns an aggreagation of opeators: All in the DB, online, registered as online, but not responsive
    pub fn get_operator_counts(&self) -> Result<(u64, u64, u64)> {
        let mut conn = self
            .pool
            .get()
            .context("get_operator_counts: failed to connect to the database")?;

        let maybe_operator_info = mm_operators
            .select(MmOperator::as_select())
            .load(&mut conn)
            .context("query get_operator_counts failed")?;

        Ok(maybe_operator_info
            .into_iter()
            .map(OperatorInfo::from)
            .fold(
                (0, 0, 0),
                |(all, mut online_, mut temporary_offline), operator| {
                    if operator.is_online() {
                        online_ += 1;
                    } else if operator.online {
                        temporary_offline += 1;
                    }

                    (all + 1, online_, temporary_offline)
                },
            ))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::database_test::TestContext;

    #[test]
    fn create_pr() {
        let _ctx = TestContext::new("postgres://postgres:postgres@127.0.0.1", "create_pr2");

        let _db =
            Database::connect_to_database("postgres://postgres:postgres@127.0.0.1/create_pr2")
                .unwrap();
    }
}
