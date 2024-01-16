use chrono::{DateTime, Duration, FixedOffset, Utc};
use sea_orm::TryIntoModel;
use serde::{Deserialize, Serialize};
use std::ops::Rem;
use uuid::Uuid;

pub mod entity;

pub type Timestamp = DateTime<Utc>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoRef(String);

impl VideoRef {
    pub fn from_url(url: &str) -> Option<VideoRef> {
        todo!()
    }
}

impl AsRef<str> for VideoRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TrackDuration {
    seconds: i64,
}

impl TrackDuration {
    pub fn new(seconds: i64) -> Self {
        Self { seconds }
    }

    pub fn can_track(self, span: Duration) -> bool {
        span % self == 0
    }

    pub fn can_track_from(self, start_at: Timestamp) -> bool {
        start_at % self == 0
    }

    pub fn stack_duration(self, n: i64) -> Duration {
        Duration::seconds(self.seconds * n)
    }
}

impl Rem<TrackDuration> for Timestamp {
    type Output = i64;

    fn rem(self, rhs: TrackDuration) -> Self::Output {
        let now = Utc::now();
        (now - self) % rhs
    }
}

impl Rem<TrackDuration> for Duration {
    type Output = i64;

    fn rem(self, rhs: TrackDuration) -> Self::Output {
        self.num_seconds() % rhs.seconds
    }
}
