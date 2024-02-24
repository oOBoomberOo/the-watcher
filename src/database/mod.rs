use std::fmt::Display;

use serde::Deserialize;
use snafu::ResultExt;
use surrealdb::opt::auth;
use surrealdb::Surreal;
use url::Url;

/// Helper trait for executing arbitrary SurrealQL queries.
pub mod query;

/// Macros for defining table methods.
pub mod macros;

use crate::error::{ApplicationError, ConnectDatabaseSnafu};
pub use crate::query;
pub use query::Query;

pub type Result<T, E = DatabaseError> = std::result::Result<T, E>;
pub type DatabaseError = surrealdb::Error;

const SETUP: &str = include_str!("../../schema.surrealql");

pub async fn connect(config: &DatabaseConfig) -> Result<(), ApplicationError> {
    database()
        .connect(config.url.as_str())
        .await
        .context(ConnectDatabaseSnafu)?;

    if let Some(credentials) = &config.credentials {
        database()
            .signin(credentials.auth())
            .await
            .context(ConnectDatabaseSnafu)?;
    }

    database()
        .query(SETUP)
        .await
        .context(ConnectDatabaseSnafu)?;

    Ok(())
}

type Database = Surreal<surrealdb::engine::any::Any>;

static DB: once_cell::sync::Lazy<Database> = once_cell::sync::Lazy::new(Database::init);

pub fn database() -> &'static impl std::ops::Deref<Target = Database> {
    &DB
}

/// Helper function for throwing a database error
pub fn throw(msg: impl Display) -> DatabaseError {
    surrealdb::error::Db::Thrown(msg.to_string()).into()
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(rename = "surreal_url")]
    url: Url,
    #[serde(flatten)]
    credentials: Option<DatabaseCredentials>,
}

#[derive(Debug, Deserialize, Clone)]
struct DatabaseCredentials {
    #[serde(rename = "surreal_db")]
    database: String,
    #[serde(rename = "surreal_ns")]
    namespace: String,
    #[serde(rename = "surreal_name")]
    username: String,
    #[serde(rename = "surreal_pass")]
    password: String,
}

impl DatabaseCredentials {
    fn auth(&self) -> impl auth::Credentials<auth::Signin, auth::Jwt> + '_ {
        auth::Database {
            database: &self.database,
            namespace: &self.namespace,
            username: &self.username,
            password: &self.password,
        }
    }
}
