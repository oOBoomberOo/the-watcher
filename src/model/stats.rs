use surrealdb::sql::Thing;

use crate::service::youtube::VideoData;

use super::*;

pub type StatsId = Thing;

pub fn new_stats_id() -> StatsId {
    stats_id(Uuid::new_v4())
}

pub fn stats_id(uuid: Uuid) -> StatsId {
    ("stats".to_string(), uuid.to_string()).into()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, new)]
pub struct Stats {
    #[new(value = "new_stats_id()")]
    pub id: StatsId,
    #[new(value = "now()")]
    pub created_at: Timestamp,
    pub tracker_id: TrackerId,
    pub video_id: VideoId,
    pub views: i64,
    pub likes: i64,
}

impl Stats {
    pub fn from_video_data(tracker: &Tracker, video_data: &VideoData) -> Self {
        Self::new(
            tracker.id.clone(),
            tracker.video_id.clone(),
            video_data.views as i64,
            video_data.likes as i64,
        )
    }
}
