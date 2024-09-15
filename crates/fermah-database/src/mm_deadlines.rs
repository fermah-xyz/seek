use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{
    dsl::{delete, insert_into},
    prelude::*,
};
use fermah_common::hash::blake3::Blake3Hash;
use thiserror::Error;

use crate::{schema::mm_deadlines::dsl::*, Database};

#[derive(Error, Debug, Clone)]
pub enum DeadlineDbError {
    #[error("{0}: failed to connect to the database")]
    FailedConnect(&'static str),
    #[error("query {0} failed")]
    QueryFailed(&'static str),
}

impl Database {
    pub fn get_nearest(&self) -> Result<Option<(Blake3Hash, DateTime<Utc>)>, DeadlineDbError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| DeadlineDbError::FailedConnect("get_nearest"))?;

        let maybe_nearest: Option<(Vec<u8>, NaiveDateTime)> = mm_deadlines
            .select((pr_id, deadline))
            .order_by(deadline.asc())
            .first(&mut conn)
            .optional()
            .map_err(|_| DeadlineDbError::QueryFailed("get_nearest"))?;

        let maybe_nearest =
            maybe_nearest.map(|(id, deadline_)| (Blake3Hash::from(id), deadline_.and_utc()));

        Ok(maybe_nearest)
    }

    pub fn add(
        &self,
        proof_request_id: Blake3Hash,
        deadline_: DateTime<Utc>,
    ) -> Result<(), DeadlineDbError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| DeadlineDbError::FailedConnect("add"))?;

        insert_into(mm_deadlines)
            .values((
                pr_id.eq(proof_request_id.as_32_bytes()),
                deadline.eq(deadline_.naive_utc()),
            ))
            .on_conflict(pr_id)
            .do_update()
            .set(deadline.eq(deadline_.naive_utc()))
            .execute(&mut conn)
            .map_err(|_| DeadlineDbError::QueryFailed("add"))?;
        Ok(())
    }

    pub fn remove(
        &self,
        proof_request_id: &Blake3Hash,
    ) -> Result<Option<DateTime<Utc>>, DeadlineDbError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| DeadlineDbError::FailedConnect("remove"))?;

        let maybe_deadline: Option<NaiveDateTime> = delete(mm_deadlines)
            .filter(pr_id.eq(proof_request_id.as_32_bytes()))
            .returning(deadline)
            .get_result(&mut conn)
            .optional()
            .map_err(|_| DeadlineDbError::QueryFailed("remove"))?;
        Ok(maybe_deadline.map(|nd| nd.and_utc()))
    }
}

#[cfg(test)]
mod tests {

    use blake3::hash;
    use chrono::Days;

    use super::*;
    use crate::database_test::TestContext;

    #[test]
    fn check_add_remove() {
        let _ctx = TestContext::new("postgres://postgres:postgres@127.0.0.1", "check_add_remove");

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_add_remove",
        )
        .unwrap();

        let proof_request_id = Blake3Hash(hash("check_add_remove".as_bytes()));
        let unknown_proof_request_id = Blake3Hash(hash("unknown_proof_request_id".as_bytes()));
        let now = Utc::now();
        assert!(db.add(proof_request_id, now).is_ok());
        assert!(matches!(db.remove(&unknown_proof_request_id), Ok(None)));

        // Postgresql timestamp has a resolution of 1 microsecond.
        assert!(
            matches!(db.remove(&proof_request_id), Ok(Some(d)) if (d - now).num_microseconds().unwrap() <= 2),
        );
    }

    #[test]
    fn check_get_nearest() {
        let _ctx = TestContext::new(
            "postgres://postgres:postgres@127.0.0.1",
            "check_get_nearest",
        );

        let db = Database::connect_to_database(
            "postgres://postgres:postgres@127.0.0.1/check_get_nearest",
        )
        .unwrap();

        let now = Utc::now();

        let test_inputs = (0..10)
            .map(|i| {
                (
                    Blake3Hash(hash(format!("check_get_nearest-{i}").as_bytes())),
                    now.checked_add_days(Days::new(i)).unwrap(),
                )
            })
            .collect::<Vec<_>>();

        test_inputs
            .iter()
            .for_each(|(id, date)| assert!(db.add(*id, *date).is_ok()));

        let (nearest_id, nearest_date) = test_inputs.first().unwrap();
        // Postgresql timestamp has a resolution of 1 microsecond.
        assert!(
            matches!(db.get_nearest(), Ok(Some((id, date))) if id == *nearest_id && (date-nearest_date).num_microseconds().unwrap() <= 2),
        );
    }
}
