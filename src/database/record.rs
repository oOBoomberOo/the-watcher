use super::Table;
use crate::prelude::*;

/// A typed record id for a database record. type `T`` must implement [Table] trait so that the table name can be inferred.
///
/// This type implements [Default] which creates a new record with a random UUID as the identifier.
pub struct Record<T> {
    inner: Thing,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Table> Record<T> {
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

    pub fn content(&self) -> String {
        self.inner.id.to_string()
    }
}

impl<T> AsRef<Thing> for Record<T> {
    fn as_ref(&self) -> &Thing {
        &self.inner
    }
}

impl<T: Table> std::default::Default for Record<T> {
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

impl<'de, T: Table> serde::Deserialize<'de> for Record<T> {
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

impl<T> std::cmp::PartialEq for Record<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> std::cmp::Eq for Record<T> {}

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
