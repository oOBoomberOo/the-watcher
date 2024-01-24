use super::*;

pub type TrackerId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, new)]
pub struct Tracker {
    #[new(value = "Uuid::new_v4()")]
    pub id: TrackerId,
    #[new(value = "now()")]
    pub created_at: Timestamp,
    #[new(value = "now()")]
    pub updated_at: Timestamp,

    pub video_id: VideoId,
    pub track_at: Timestamp,
    pub track_duration: TrackDuration,
    #[new(default)]
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
        self.track_at + offset
    }

    pub fn has_reached_target(&self, stats: &Stats) -> bool {
        self.track_target
            .map_or(false, |target| stats.views >= target)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct TrackDuration(pub std::time::Duration);

impl TrackDuration {
    pub fn seconds(self) -> i64 {
        self.0.as_secs() as i64
    }

    pub fn round_up_from(self, duration: Duration) -> Duration {
        let sec = self.seconds();
        let n = duration.num_seconds() / sec;
        Duration::seconds(sec * (n + 1))
    }
}
