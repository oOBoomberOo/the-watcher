use std::num::NonZeroI64;

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
    pub track_target: Option<NonZeroI64>,
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
            .map_or(false, |target| stats.views >= target.get())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct TrackDuration(i64);

impl TrackDuration {
    pub fn round_up_from(self, duration: Duration) -> Duration {
        let n = duration.num_seconds() / self.0;
        Duration::seconds(self.0 * (n + 1))
    }
}
