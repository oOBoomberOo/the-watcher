use chrono::Duration;
use derive_more::{AsRef, Deref, From};
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::ops::Sub;

pub fn now() -> Timestamp {
    chrono::Utc::now().into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, new, From, Deref, AsRef)]
pub struct Timestamp(chrono::DateTime<chrono::Utc>);

impl Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.to_rfc3339().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        chrono::DateTime::parse_from_rfc3339(&s)
            .map(|dt| Self(dt.into()))
            .map_err(serde::de::Error::custom)
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}
