use std::ops::Neg;

use chrono::Utc;
use tokio::time::Instant;

use crate::prelude::*;

/// A wrapper around [chrono::DateTime] that implemented a default value to the current time.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize, From,
)]
pub struct Timestamp(pub chrono::DateTime<Utc>);

impl Timestamp {
    pub fn now() -> Self {
        Utc::now().into()
    }

    /// Convert [Timestamp] into [Instant].
    ///
    /// ## Note
    /// There is no direct way to convert arbitrary date time into [Instant] because of its monotonic guarantee.
    /// Instead, we calculate the elapsed time from the current time and then add or subtract it from the current instant.
    pub fn to_instant(self) -> Instant {
        let elapsed = Utc::now().signed_duration_since(self.0);
        let now = Instant::now();

        // [std::time::Duration] cannot be negative, so we need to handle it manually.
        let past_instant = elapsed.to_std().map(|duration| now - duration);

        let future_instant = now + elapsed.neg().to_std().unwrap_or_default();

        past_instant.unwrap_or(future_instant)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl std::ops::Deref for Timestamp {
    type Target = chrono::DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Timestamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::convert::AsRef<chrono::DateTime<Utc>> for Timestamp {
    fn as_ref(&self) -> &chrono::DateTime<Utc> {
        &self.0
    }
}

impl std::ops::Sub<Timestamp> for Timestamp {
    type Output = chrono::Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}
