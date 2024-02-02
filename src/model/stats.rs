use super::*;
pub type StatsId = Record<Stats>;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, new)]
pub struct Stats {
    #[new(default)]
    pub id: StatsId,
    #[new(value = "now()")]
    pub created_at: Timestamp,
    pub tracker_id: TrackerId,
    pub video_id: VideoId,
    pub views: i64,
    pub likes: i64,
}
