use super::*;

use surrealdb::sql::Thing;

pub type TrackerId = Thing;

pub fn new_tracker_id() -> TrackerId {
    tracker_id(Uuid::new_v4())
}

pub fn tracker_id(uuid: Uuid) -> TrackerId {
    ("trackers".to_string(), uuid.to_string()).into()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, new)]
pub struct Tracker {
    #[new(value = "new_tracker_id()")]
    pub id: TrackerId,
    #[new(value = "now()")]
    pub created_at: Timestamp,
    #[new(value = "now()")]
    pub updated_at: Timestamp,

    pub video_id: VideoId,
    pub track_at: Timestamp,
    pub track_duration: TrackDuration,
    #[serde(default)]
    pub track_target: Option<i64>,
    #[new(value = "true")]
    pub active: bool,
}

impl Tracker {
    pub fn get_next_timestamp(&self, now: Timestamp) -> Timestamp {
        if self.track_at > now {
            return self.track_at;
        }

        let offset = self.track_duration.round_up_from(now - self.track_at);
        (*self.track_at + offset).into()
    }

    pub fn has_reached_target(&self, stats: &Stats) -> bool {
        self.track_target
            .map_or(false, |target| stats.views >= target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct TrackDuration(
    #[serde(
        serialize_with = "TrackDuration::serializer",
        deserialize_with = "TrackDuration::deserializer"
    )]
    pub std::time::Duration,
);

impl TrackDuration {
    pub fn from_seconds(seconds: i64) -> Self {
        Self(std::time::Duration::from_secs(seconds as u64))
    }

    pub fn seconds(self) -> i64 {
        self.0.as_secs() as i64
    }

    pub fn round_up_from(self, duration: Duration) -> Duration {
        let sec = self.seconds();
        let n = duration.num_seconds() / sec;
        Duration::seconds(sec * (n + 1))
    }

    pub fn duration(self) -> Duration {
        Duration::seconds(self.seconds())
    }

    pub fn serializer<S>(duration: &std::time::Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(duration.as_secs() as i64)
    }

    pub fn deserializer<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let seconds = i64::deserialize(deserializer)?;
        Ok(std::time::Duration::from_secs(seconds as u64))
    }
}
