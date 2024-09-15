use std::{collections::HashSet, net::SocketAddr};

use anyhow::{Context, Result};
use diesel::{dsl::insert_into, prelude::*, update};
use ethers::types::U256;
use fermah_common::operator::OperatorId;

use crate::{
    models::{EthAddress, EthU256},
    schema,
    Database,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperatorParams {
    pub socket: Option<SocketAddr>,
    pub is_el_registered: bool,

    pub registered_till_block: U256,
    // Operator status
}

impl Database {
    /// Function in DB, which checks if the operator is in the system, returning `Some(..)`, and if it is available `Some(true)`
    pub fn is_existing_prover(&self, operator_id: &OperatorId) -> Result<bool> {
        use schema::avs_operators::dsl::*;

        let mut conn = self
            .pool
            .get()
            .context("is_existing_prover: failed to connect to the database")?;

        let n = avs_operators
            .select(id)
            .filter(id.eq(EthAddress::from(*operator_id)))
            .filter(socket.is_not_null().and(is_el_registered.eq(true)))
            .execute(&mut conn)
            .context("query is_existing_prover failed")?;

        Ok(n == 1)
    }

    pub fn get_ready_provers(&self) -> Result<HashSet<(OperatorId, OperatorParams)>> {
        use schema::avs_operators::dsl;

        let mut conn = self
            .pool
            .get()
            .context("get_ready_provers: failed to connect to the database")?;

        let operators: Vec<_> = schema::avs_operators::table
            .select((
                dsl::id,
                dsl::socket,
                dsl::is_el_registered,
                dsl::registered_till_block,
            ))
            .filter(
                dsl::socket
                    .is_not_null()
                    .and(dsl::is_el_registered.eq(true)),
            )
            .load::<(EthAddress, Option<String>, bool, EthU256)>(&mut conn)
            .inspect_err(|e| tracing::error!(?e, "sorry"))
            .context("query get_ready_provers failed")?;

        Ok(operators
            .into_iter()
            .map(|(id, socket, is_el_registered, registered_till_block)| {
                (
                    id.into(),
                    OperatorParams {
                        socket: socket.map(|s| s.parse().unwrap()),
                        is_el_registered,
                        registered_till_block: registered_till_block.into(),
                    },
                )
            })
            .collect())
    }

    pub fn register_operator_from_el(&self, operator_id: OperatorId) -> Result<()> {
        use schema::avs_operators::dsl::*;

        let mut conn = self
            .pool
            .get()
            .context("register_operator_from_el: failed to connect to the database")?;

        insert_into(avs_operators)
            .values((
                id.eq(EthAddress::from(operator_id)),
                is_el_registered.eq(true),
            ))
            .on_conflict(id)
            .do_update()
            .set((
                id.eq(EthAddress::from(operator_id)),
                is_el_registered.eq(true),
            ))
            .execute(&mut conn)
            .context("query register_operator_from_el failed")?;

        Ok(())
    }
    //
    pub fn deregister_operator_from_el(&self, operator_id: &OperatorId) -> Result<()> {
        use schema::avs_operators::dsl::*;

        let mut conn = self
            .pool
            .get()
            .context("deregister_operator_from_el: failed to connect to the database")?;

        update(avs_operators)
            .filter(id.eq(EthAddress::from(*operator_id)))
            .set(is_el_registered.eq(false))
            .execute(&mut conn)
            .context("query deregister_operator_from_el failed")?;

        Ok(())
    }
    //

    pub fn operator_update_socket(
        &self,
        operator_id: &OperatorId,
        socket_: SocketAddr,
    ) -> Result<()> {
        use schema::avs_operators::dsl::*;

        let mut conn = self
            .pool
            .get()
            .context("operator_update_socket: failed to connect to the database")?;

        insert_into(avs_operators)
            .values((
                id.eq(EthAddress::from(*operator_id)),
                socket.eq(socket_.to_string()),
            ))
            .on_conflict(id)
            .do_update()
            .set((
                id.eq(EthAddress::from(*operator_id)),
                socket.eq(socket_.to_string()),
            ))
            .execute(&mut conn)
            .context("query operator_update_socket failed")?;

        Ok(())
    }

    pub fn set_operator_registered_till(
        &self,
        operator: &OperatorId,
        registered_till_block_: U256,
    ) -> Result<()> {
        use schema::avs_operators::dsl::*;

        let mut conn = self
            .pool
            .get()
            .context("set_operator_registered_till: failed to connect to the database")?;

        insert_into(avs_operators)
            .values((
                id.eq(EthAddress::from(*operator)),
                registered_till_block.eq(EthU256::from(registered_till_block_)),
            ))
            .on_conflict(id)
            .do_update()
            .set((
                id.eq(EthAddress::from(*operator)),
                registered_till_block.eq(EthU256::from(registered_till_block_)),
            ))
            .execute(&mut conn)
            .context("queryset_operator_registered_till failed")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use ethers::types::Address;

    use super::*;
    use crate::database_test::TestContext;

    #[test]
    fn check_one_operator() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_one_operator",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_one_operator",
        )
        .unwrap();
        let operator_id = Address::random().into();
        let socket = "127.0.0.1:8080".parse().unwrap();

        assert!(matches!(db.is_existing_prover(&operator_id), Ok(false)));
        assert!(matches!(db.get_ready_provers(), Ok(hs) if hs.is_empty()));

        let res = db.operator_update_socket(&operator_id, socket);
        assert!(res.is_ok());
        assert!(matches!(db.is_existing_prover(&operator_id), Ok(false)));
        assert!(
            matches!(db.get_ready_provers(), Ok(hs) if hs.is_empty()),
            "expected empty, got: {:?}",
            db.get_ready_provers()
        );

        let res = db.register_operator_from_el(operator_id);
        assert!(res.is_ok());
        assert!(matches!(db.is_existing_prover(&operator_id), Ok(true)));
        assert!(matches!(db.get_ready_provers(), Ok(hs) if hs.len() == 1));

        for (oid, params) in db.get_ready_provers().unwrap().drain() {
            assert_eq!(oid, operator_id);
            assert_eq!(params.registered_till_block, U256::zero());
            assert!(params.is_el_registered);
            assert_eq!(params.socket, Some(socket));
        }

        let registration_till_block = U256::from_dec_str("123456").unwrap();
        let res = db.set_operator_registered_till(&operator_id, registration_till_block);
        assert!(res.is_ok());
        for (oid, params) in db.get_ready_provers().unwrap().drain() {
            assert_eq!(oid, operator_id);
            assert_eq!(params.registered_till_block, registration_till_block);
            assert!(params.is_el_registered);
            assert_eq!(params.socket, Some(socket));
        }

        db.deregister_operator_from_el(&operator_id).unwrap();
        assert!(matches!(db.is_existing_prover(&operator_id), Ok(false)));
        assert!(matches!(db.get_ready_provers(), Ok(hs) if hs.is_empty()));
    }

    #[test]
    fn check_multiple_operators() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_multiple_operators",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_multiple_operators",
        )
        .unwrap();
        let operators = (0..5)
            .map(|i| {
                (
                    Address::random().into(),
                    format!("127.0.0.1:808{i}").parse().unwrap(),
                )
            })
            .collect::<Vec<(OperatorId, SocketAddr)>>();

        for (i, (operator_id, socket)) in operators.into_iter().enumerate() {
            assert!(
                matches!(db.get_ready_provers(), Ok(hs) if hs.len()==i),
                "expected {i}, operator_id={operator_id:?}, got: {:?}",
                db.get_ready_provers()
            );
            assert!(matches!(db.is_existing_prover(&operator_id), Ok(false)));
            assert!(db.operator_update_socket(&operator_id, socket).is_ok());
            assert!(db.register_operator_from_el(operator_id).is_ok());
            assert!(matches!(db.is_existing_prover(&operator_id), Ok(true)));
        }
    }
}
