use self::service::youtube::VideoInfo;
use super::*;

pub type TrackerId = Record<Tracker>;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, new)]
pub struct Tracker {
    #[new(default)]
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

    pub fn create_stats(&self, video_info: VideoInfo) -> Stats {
        Stats::new(self.id.clone(), video_info.id, video_info.views, video_info.likes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackDuration(pub std::time::Duration);

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
}

impl Serialize for TrackDuration {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.0.as_secs() as i64)
    }
}

impl<'de> Deserialize<'de> for TrackDuration {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let seconds = i64::deserialize(deserializer)?;
        Ok(TrackDuration::from_seconds(seconds))
    }
}
