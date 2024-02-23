use std::ops::Deref;

use futures::Future;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use surrealdb::opt::QueryResult;

use super::*;

/// Helper trait for conveniently fetching a database query and extract the first result.
pub trait Query {
    fn fetch<T: DeserializeOwned>(self) -> impl Future<Output = super::Result<T>>
    where
        usize: QueryResult<T>;
}

impl<'r, C: surrealdb::Connection> Query for surrealdb::method::Query<'r, C> {
    async fn fetch<T: DeserializeOwned>(self) -> super::Result<T>
    where
        usize: QueryResult<T>,
    {
        self.await?.take::<T>(0)
    }
}

/// Query result extractor that allows exactly one value to be returned.
#[derive(Debug, Deserialize)]
pub struct Only<T>(pub T);

impl<T: DeserializeOwned> QueryResult<Only<T>> for usize {
    fn query_result(self, response: &mut surrealdb::Response) -> super::Result<Only<T>> {
        let response: Vec<T> = self.query_result(response)?;
        response.try_into()
    }
}

impl<T> TryFrom<Option<T>> for Only<T> {
    type Error = DatabaseError;

    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        value
            .ok_or_else(|| super::throw("expected exactly one result, but got nothing"))
            .map(Only)
    }
}

impl<T> TryFrom<Vec<T>> for Only<T> {
    type Error = DatabaseError;

    fn try_from(mut value: Vec<T>) -> Result<Self, Self::Error> {
        match value.len() {
            0 => Err(super::throw("expected exactly one result, but got none")),
            1 => Ok(Only(value.remove(0))),
            _ => Err(super::throw("expected exactly one result, but got more")),
        }
    }
}

impl<T> Deref for Only<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
