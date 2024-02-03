use std::collections::HashMap;

use derive_new::new;
use snafu::{Location, OptionExt as _, ResultExt as _, Snafu};
use surrealdb::{
    engine::any::Any,
    opt::{
        auth::{self, Credentials, Jwt, Signin},
        IntoQuery, IntoResource, QueryResult,
    },
    Surreal,
};
use url::Url;

pub use surrealdb::sql::Thing;

use crate::Located;
pub type Result<T, E = DatabaseError> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DatabaseError {
    #[snafu(display("failed to query the database at {location}: {source}"))]
    DatabaseQuery {
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to deserialize the database response at {location}: {source}"))]
    DatabaseDeserialize {
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to parse the database response at {location}: response is empty"))]
    EmptyQuery {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("cannot connect to the database `{url}` at {location}: {source}"))]
    DatabaseConnection {
        url: Url,
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("url `{url}` is missing a namespace parameter (ns) at {location}"))]
    NoNamespace {
        url: Url,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("url `{url}` is missing a database parameter (db) at {location}"))]
    NoDatabase {
        url: Url,
        #[snafu(implicit)]
        location: Location,
    },
}

impl Located for DatabaseError {
    fn location(&self) -> Location {
        match self {
            DatabaseError::DatabaseQuery { location, .. }
            | DatabaseError::DatabaseDeserialize { location, .. }
            | DatabaseError::EmptyQuery { location, .. }
            | DatabaseError::DatabaseConnection { location, .. }
            | DatabaseError::NoNamespace { location, .. }
            | DatabaseError::NoDatabase { location, .. } => *location,
        }
    }
}

/// Represents an identifier for a database record.
pub trait Id {
    /// Returns the ID of the record.
    fn id(&self) -> &Thing;

    /// Returns the name of the table associated with the record.
    fn table() -> &'static str;
}

impl<T: Id> Id for &T {
    fn id(&self) -> &Thing {
        (*self).id()
    }

    fn table() -> &'static str {
        T::table()
    }
}

/// Represents a type that can be used to establish a connection to a database.
pub trait Connection {
    /// The type of the connected database.
    type Database;

    /// Establishes a connection to the database.
    fn connect(&self) -> impl std::future::Future<Output = Result<Self::Database>> + Send;
}

impl Connection for Url {
    type Database = Surreal<Any>;

    /// Connects to the database using the URL. The URL must contain the namespace and database via the `ns` and `db` query parameters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use url::Url;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let url = Url::parse("http://localhost:8080?ns=example&db=example").unwrap();
    ///    let db = url.connect().await.unwrap();
    /// }
    /// ```
    async fn connect(&self) -> Result<Self::Database> {
        let db = surrealdb::engine::any::connect(self.as_str())
            .await
            .context(DatabaseConnectionSnafu { url: self.clone() })?;

        let auth = self.as_credentials()?;
        db.signin(auth.to_raw())
            .await
            .context(DatabaseConnectionSnafu { url: self.clone() })?;

        todo!()
    }
}

/// A trait for converting a type into credentials.
pub trait AsCredentials {
    /// The associated type for the credentials.
    type Credentials;

    /// Converts the type into credentials.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the converted credentials or an error if the conversion fails.
    fn as_credentials(&self) -> Result<Self::Credentials>;
}

impl AsCredentials for Url {
    type Credentials = Auth;

    fn as_credentials(&self) -> Result<Self::Credentials> {
        let username = self.username().to_owned();
        let password = self.password().unwrap_or("").to_owned();

        let mut query: HashMap<String, String> = self
            .query_pairs()
            .map(|(key, val)| (key.to_string(), val.to_string()))
            .collect();

        let namespace = query
            .remove("ns")
            .context(NoNamespaceSnafu { url: self.clone() })?;

        let database = query
            .remove("db")
            .context(NoDatabaseSnafu { url: self.clone() })?;

        Ok(Auth {
            username,
            password,
            namespace,
            database,
        })
    }
}

/// Represents authentication information for a database connection.
#[derive(Debug, Clone, PartialEq, Eq, new)]
pub struct Auth {
    pub username: String,
    pub password: String,
    pub namespace: String,
    pub database: String,
}

impl Auth {
    pub fn to_raw(&self) -> impl Credentials<Signin, Jwt> + '_ {
        auth::Database {
            username: &self.username,
            password: &self.password,
            namespace: &self.namespace,
            database: &self.database,
        }
    }
}

/// Represents a database wrapper.
///
/// This struct provides a wrapper around a database, allowing for easier interaction and abstraction.
#[derive(Debug, Clone, new)]
pub struct Database {
    database: Surreal<Any>,
    // ...
}

impl Database {
    pub async fn connect(url: Url) -> Result<Self> {
        url.connect().await.map(Database::new)
    }

    /// Create a builder to execute arbitrary SQL code on the database.
    ///
    /// # Example
    ///
    /// ```rust
    /// let db = Database::connect("http://localhost:8080?ns=example&db=example").await.unwrap();
    /// let result: Vec<HololiveMember> = db.sql("SELECT name, height, subscriber_count FROM hololive WHERE height < $height AND subscriber_count > $subscribers")
    ///                 .bind(("height", 160))
    ///                 .bind(("subscribers", 1_000_000))
    ///                 .fetch().await?;
    /// ```
    ///
    /// The `fetch` method can deserialize the result into either a single value (`Option<T>`) or a collection of values (`Vec<T>`).
    pub fn sql(&self, query: impl IntoQuery) -> Query<'_> {
        let query = self.database.query(query);
        Query { query }
    }
}

impl std::ops::Deref for Database {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.database
    }
}

#[derive(Debug)]
pub struct Query<'a> {
    query: surrealdb::method::Query<'a, surrealdb::engine::any::Any>,
}

impl Query<'_> {
    pub fn bind(mut self, params: impl serde::Serialize) -> Self {
        let query = self.query;
        self.query = query.bind(params);
        self
    }

    pub async fn fetch<T: serde::de::DeserializeOwned>(self) -> Result<T>
    where
        usize: QueryResult<T>,
    {
        let mut statements = self.query.await.context(DatabaseQuerySnafu)?;
        let result = statements.take::<T>(0).context(DatabaseDeserializeSnafu)?;
        Ok(result)
    }
}

/// A typed record id for a database record. type `T`` must implement [Id] trait so that the table name can be inferred.
///
/// This type implements [Default] which creates a new record with a random UUID as the identifier.
#[derive(PartialEq, Eq)]
pub struct Record<T> {
    inner: Thing,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Id> Record<T> {
    /// Creates a new `Record` from the specified `id` and inferred the table's name from `T`.
    pub fn new(id: impl Into<surrealdb::sql::Id>) -> Self {
        let inner = Thing {
            tb: T::table().to_string(),
            id: id.into(),
        };

        Record {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new `Record` with a random UUID as the identifier.
    pub fn uuid() -> Self {
        Self::new(surrealdb::sql::Id::uuid())
    }
}

impl<T: Id> std::default::Default for Record<T> {
    fn default() -> Self {
        Self::uuid()
    }
}

impl<T> std::ops::Deref for Record<T> {
    type Target = Thing;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> std::fmt::Debug for Record<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> std::fmt::Display for Record<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> std::clone::Clone for Record<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> serde::Serialize for Record<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, T: Id> serde::Deserialize<'de> for Record<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let thing = Thing::deserialize(deserializer)?;

        let expected = T::table();
        let actual = &thing.tb;

        if expected == actual {
            return Err(serde::de::Error::custom(format!(
                "table name mismatch, expected '{expected}' but got '{actual}'"
            )));
        }

        Ok(Record {
            inner: thing,
            _marker: std::marker::PhantomData,
        })
    }
}

impl<T> std::hash::Hash for Record<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T, R> IntoResource<R> for Record<T>
where
    Thing: IntoResource<R>,
{
    fn into_resource(self) -> std::result::Result<surrealdb::opt::Resource, surrealdb::Error> {
        self.inner.into_resource()
    }
}
