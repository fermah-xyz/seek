use std::net::SocketAddr;

use anyhow::{Context, Result};
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use ethers::types::U256;

pub mod avs_operators;
pub mod avs_proof_requesters;
pub mod mm_deadlines;
pub mod mm_operators;
pub mod mm_proof_requests;
pub mod models;
pub mod schema;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperatorParams {
    pub socket: Option<SocketAddr>,
    pub is_el_registered: bool,

    pub registered_till_block: U256,
    // Operator status
}

pub(crate) type DbConnection =
    diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>;

#[derive(Clone, Debug)]
pub struct Database {
    pool: Pool<ConnectionManager<PgConnection>>,
}
impl Database {
    /// Connect to the database with the provided URL
    pub fn connect_to_database(database_url: &str) -> Result<Database> {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .test_on_check_out(true)
            .build(manager)
            .context(": failed to connect to the database")?;
        Ok(Self { pool })
    }
}

#[cfg(any(test, feature = "database_test"))]
pub mod database_test {
    use diesel::{Connection, PgConnection, RunQueryDsl};
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    pub struct TestContext {
        base_url: String,
        db_name: String,
    }

    impl TestContext {
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

        pub fn new(base_url: &str, db_name: &str) -> Self {
            let postgres_url = format!("{base_url}/postgres");
            let mut conn = PgConnection::establish(&postgres_url)
                .expect("Cannot connect to postgres database.");

            // Ignore error if unable to create DB, it might exist already
            let _ =
                diesel::sql_query(format!("CREATE DATABASE {db_name}").as_str()).execute(&mut conn);

            let mut test_conn = PgConnection::establish(&format!("{base_url}/{db_name}"))
                .expect("Cannot connect to test database.");

            // Ignore error, migrations might be already ran
            let _ = test_conn.run_pending_migrations(Self::MIGRATIONS);

            Self {
                base_url: base_url.to_string(),
                db_name: db_name.to_string(),
            }
        }
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            let postgres_url = format!("{}/postgres", self.base_url);
            let mut conn = PgConnection::establish(&postgres_url)
                .expect("Cannot connect to postgres database.");

            let disconnect_users = format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}';",
                self.db_name
            );

            diesel::sql_query(disconnect_users.as_str())
                .execute(&mut conn)
                .unwrap();

            let query = diesel::sql_query(format!("DROP DATABASE {}", self.db_name).as_str());
            query
                .execute(&mut conn)
                .unwrap_or_else(|_| panic!("Couldn't drop database {}", self.db_name));
        }
    }
}
