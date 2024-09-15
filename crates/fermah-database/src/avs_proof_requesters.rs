use anyhow::{Context, Result};
use diesel::{dsl::insert_into, prelude::*};
use ethers::types::{Address, U256};

use crate::{
    models::{EthAddress, EthU256},
    schema,
    Database,
};

impl Database {
    pub fn get_seeker_deposit(&self, proof_requester: &Address) -> Result<Option<U256>> {
        use schema::avs_proof_requesters::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("get_seeker_deposit: failed to connect to the database")?;

        let maybe_deposit = avs_proof_requesters
            .select(deposit)
            .filter(id.eq(EthAddress::from(*proof_requester)))
            .first::<EthU256>(&mut conn)
            .map(|d| d.into())
            .optional()
            .context("query get_seeker_deposit failed")?;

        Ok(maybe_deposit)
    }

    pub fn get_seekers_amount(&self) -> Result<u64> {
        use schema::avs_proof_requesters::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("get_seeker_deposit: failed to connect to the database")?;

        let n_seekers: i64 = avs_proof_requesters
            .count()
            .first(&mut conn)
            .context("query get_seekers_amount failed")?;

        Ok(n_seekers as u64)
    }

    pub fn set_proof_requester_deposit(
        &self,
        proof_requester: &Address,
        deposit_: U256,
    ) -> Result<()> {
        use schema::avs_proof_requesters::dsl::*;
        let mut conn = self
            .pool
            .get()
            .context("set_proof_requester_deposit: failed to connect to the database")?;

        let proof_requester = EthAddress::from(*proof_requester);
        let deposit_ = EthU256::from(deposit_);

        insert_into(avs_proof_requesters)
            .values((id.eq(proof_requester), deposit.eq(deposit_)))
            .on_conflict(id)
            .do_update()
            .set((id.eq(proof_requester), deposit.eq(deposit_)))
            .execute(&mut conn)
            .context("query set_proof_requester_deposit failed")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::database_test::TestContext;
    #[test]
    fn check_upsert_deposit() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_upsert_deposit",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_upsert_deposit",
        )
        .unwrap();
        let proof_requester = Address::random();
        let initial_deposit = U256::from_dec_str("123456789000000").unwrap();
        let new_deposit = U256::from_dec_str("12345").unwrap();

        let maybe_deposit = db.get_seeker_deposit(&proof_requester);
        assert!(matches!(maybe_deposit, Ok(None)), "{maybe_deposit:?}");

        let res = db.set_proof_requester_deposit(&proof_requester, initial_deposit);
        assert!(res.is_ok());

        let maybe_deposit = db.get_seeker_deposit(&proof_requester);
        assert!(
            matches!(maybe_deposit, Ok(Some(d)) if d == initial_deposit),
            "insert deposit failed: {maybe_deposit:?}"
        );

        let res = db.set_proof_requester_deposit(&proof_requester, new_deposit);
        assert!(res.is_ok());
        let maybe_deposit = db.get_seeker_deposit(&proof_requester);
        assert!(
            matches!(maybe_deposit, Ok(Some(d)) if d == new_deposit),
            "insert deposit failed: {maybe_deposit:?}"
        );
    }

    #[test]
    fn check_get_seekers_amount() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_get_seekers_amount",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_get_seekers_amount",
        )
        .unwrap();

        let n_seekers = 12;

        for i in 0..n_seekers {
            let proof_requester = Address::random();
            let initial_deposit = U256::from_dec_str("123456789000000").unwrap() * i;

            assert!(db
                .set_proof_requester_deposit(&proof_requester, initial_deposit)
                .is_ok());
        }

        assert!(matches!(db.get_seekers_amount(), Ok(n) if n == n_seekers));
    }
}
