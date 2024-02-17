use std::marker::PhantomData;
use surrealdb::engine::any::Any;

use crate::prelude::*;

/// Helper trait for executing arbitrary SurrealQL queries.
pub mod query;

/// Typed record identifier for a database record.
pub mod record;

/// Macros for defining table methods.
pub mod macros;

pub mod prelude {
    pub use super::query::{Only, Sql};
    pub use super::record::*;
    pub use super::{Connection, Database, IntoDatabase, SurrealTokenConfig, Table};
    pub use super::{DatabaseConnectionError, DatabaseQueryError};

    pub use crate::{define_crud, define_relation, define_table};
}

/// Describe all possible errors that can occur when connecting to the database.
#[derive(Debug, Snafu)]
pub enum DatabaseConnectionError {
    #[snafu(display("failed to connect to the database at {url}: {location}"))]
    Connection {
        url: Url,
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to login to the database with (username={username},password={password}) at {location}"))]
    Unauthorized {
        url: Url,
        username: String,
        password: String,

        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("missing a namespace parameter"))]
    MissingNamespace {
        url: Url,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing a database parameter"))]
    MissingDatabase {
        url: Url,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to define JWT token at {location}"))]
    DefineToken {
        config: SurrealTokenConfig,

        source: DatabaseQueryError,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Describe all possible errors that can occur when querying the database.
#[derive(Debug, Snafu)]
pub enum DatabaseQueryError {
    #[snafu(display("failed to deserialize the database response at {location}"))]
    Deserialize {
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("query could not be constructed at {location}"))]
    MalformedQuery {
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("expected exactly one result, but got none at {location}"))]
    NoResults {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("expected exactly one result, but got more than one at {location}"))]
    TooManyResults {
        #[snafu(implicit)]
        location: Location,
    },
}

impl From<surrealdb::Error> for DatabaseQueryError {
    #[track_caller]
    fn from(value: surrealdb::Error) -> Self {
        Self::Deserialize {
            source: value,
            location: Location::default(),
        }
    }
}

/// Represents a database table. [Table::id] must return the record's ID that uniquely identifies the record.
pub trait Table {
    /// Returns the ID of the record.
    fn id(&self) -> &Thing;

    /// Returns the name of the table associated with the record.
    fn table() -> &'static str;

    fn resource() -> Resource<Self> {
        Resource(PhantomData)
    }
}

impl<T: Table> Table for &T {
    fn id(&self) -> &Thing {
        (*self).id()
    }

    fn table() -> &'static str {
        T::table()
    }
}

pub struct Resource<T: ?Sized>(PhantomData<T>);

impl<T: Table> IntoResource<Vec<T>> for Resource<T> {
    fn into_resource(self) -> surrealdb::Result<surrealdb::opt::Resource> {
        let table = surrealdb::sql::Table(T::table().into());

        <surrealdb::sql::Table as IntoResource<Vec<T>>>::into_resource(table)
    }
}

/// Represents a type that can be used to establish a connection to the database.
pub trait Connection {
    /// Establishes a connection to the database.
    fn connect(
        &self,
    ) -> impl std::future::Future<Output = Result<Database, DatabaseConnectionError>> + Send;
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerConnection<'a> {
    pub address: &'a Url,
    pub namespace: &'a str,
    pub database: &'a str,
    pub username: &'a str,
    pub password: &'a str,
}

impl Connection for ServerConnection<'_> {
    async fn connect(&self) -> Result<Database, DatabaseConnectionError> {
        let url = self.address.clone();
        let db = surrealdb::engine::any::connect(url.as_str())
            .await
            .context(ConnectionSnafu { url: url.clone() })?;

        let credentials = surrealdb::opt::auth::Database {
            namespace: self.namespace,
            database: self.database,
            username: self.username,
            password: self.password,
        };

        db.signin(credentials).await.context(UnauthorizedSnafu {
            username: self.username,
            password: self.password,
            url,
        })?;

        Ok(Database::new(db))
    }
}

/// A database wrapper.
#[derive(Debug, Clone, new)]
pub struct Database {
    database: Surreal<Any>,
}

pub trait IntoDatabase {
    #[allow(clippy::wrong_self_convention)]
    #[must_use]
    fn into_database(&self) -> &Surreal<Any>;
}

impl IntoDatabase for Database {
    fn into_database(&self) -> &Surreal<Any> {
        &self.database
    }
}

impl IntoDatabase for Surreal<Any> {
    fn into_database(&self) -> &Surreal<Any> {
        self
    }
}

impl IntoDatabase for axum::extract::State<Database> {
    fn into_database(&self) -> &Surreal<Any> {
        &self.database
    }
}

impl IntoDatabase for std::sync::Arc<Database> {
    fn into_database(&self) -> &Surreal<Any> {
        &self.database
    }
}

impl<D: IntoDatabase> IntoDatabase for &D {
    fn into_database(&self) -> &Surreal<Any> {
        (*self).into_database()
    }
}

impl std::ops::Deref for Database {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.database
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SurrealTokenConfig {
    #[serde(rename = "surreal_token")]
    pub name: String,
    #[serde(rename = "surreal_scope")]
    pub scope: String,
    #[serde(rename = "surreal_token_secret")]
    pub token: String,
    #[serde(rename = "surreal_token_algorithm")]
    pub algorithm: jsonwebtoken::Algorithm,
}

impl SurrealTokenConfig {
    pub async fn setup_token(self, database: &Database) -> Result<(), DatabaseConnectionError> {
        database
            .sql("DEFINE TOKEN $name ON SCOPE $scope TYPE $algorithm VALUE $token")
            .bind(&self)
            .execute()
            .await
            .context(DefineTokenSnafu { config: self })?;
        Ok(())
    }
}
