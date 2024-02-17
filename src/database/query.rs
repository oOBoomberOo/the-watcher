use std::ops::Deref;

use surrealdb::opt::QueryResult;

use super::*;

/// An extension trait that allows you to execute raw SQL queries. Parameters can be bound using the [bind] method which takes any serializable data structure.
///
/// # Example
/// ```
/// let post_written_by_fubuki: Vec<Post> = database.sql("SELECT * FROM posts WHERE author = $user")
///     .bind(("user", "users:fubuki"))
///     .fetch()
///     .await?;
/// ```
///
/// This trait is implemented for any type that implements [IntoDatabase].
pub trait Sql<'a> {
    fn sql(&'a self, query: &str) -> Bindings<'a>;
}

impl<'a, Database> Sql<'a> for Database
where
    Database: IntoDatabase,
{
    fn sql(&'a self, query: &str) -> Bindings<'a> {
        let query = self.into_database().query(query);
        Bindings::new(query)
    }
}

#[derive(Debug, new)]
pub struct Bindings<'a> {
    query: surrealdb::method::Query<'a, surrealdb::engine::any::Any>,
}

impl Bindings<'_> {
    pub fn bind(mut self, params: impl serde::Serialize) -> Self {
        let query = self.query;
        self.query = query.bind(params);
        self
    }

    /// Execute the query and return a [surrealdb::Response] which is SurrealDB's way to represent a list of statements returned from the database.
    ///
    /// This means that you can execute multiple queries in a single call and get all the results back.
    pub async fn execute(self) -> Result<surrealdb::Response, DatabaseQueryError> {
        let response = self.query.await.context(MalformedQuerySnafu)?;
        tracing::debug!(?response, "executed query");
        Ok(response)
    }

    /// Execute the queries and deserialize all the results into a list of list of values.
    pub async fn fetch_all<T: DeserializeOwned>(self) -> Result<Vec<T>, DatabaseQueryError>
    where
        usize: QueryResult<T>,
    {
        let mut statements = self.execute().await?;
        let size = statements.num_statements();

        let mut results = Vec::with_capacity(size);
        for i in 0..size {
            let result = statements.take::<T>(i).context(DeserializeSnafu)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute the query and return the first result as a deserialized value.
    pub async fn fetch_first<T: DeserializeOwned>(self) -> Result<T, DatabaseQueryError>
    where
        usize: QueryResult<T>,
    {
        let mut statements = self.execute().await?;
        let result = statements.take::<T>(0).context(DeserializeSnafu)?;
        Ok(result)
    }

    pub async fn fetch_one<T: DeserializeOwned>(self) -> Result<T, DatabaseQueryError>
    where
        usize: QueryResult<T>,
    {
        self.fetch_first::<Option<T>>()
            .await?
            .context(NoResultsSnafu)
    }
}

#[derive(Debug, Deserialize)]
pub struct Only<T>(pub T);

impl<T: DeserializeOwned> QueryResult<Only<T>> for usize {
    fn query_result(self, response: &mut surrealdb::Response) -> surrealdb::Result<Only<T>> {
        let response: Option<T> = self.query_result(response)?;

        response.map(Only).ok_or_else(|| {
            surrealdb::error::Api::ParseError("expected exactly one result, but got none".into())
                .into()
        })
    }
}

impl<T> TryFrom<Vec<T>> for Only<T> {
    type Error = DatabaseQueryError;

    fn try_from(mut value: Vec<T>) -> Result<Self, Self::Error> {
        match value.len() {
            0 => Err(NoResultsSnafu.build()),
            1 => Ok(Only(value.remove(0))),
            _ => Err(TooManyResultsSnafu.build()),
        }
    }
}

impl<T> Deref for Only<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
