use crate::model::{MilestoneId, Timestamp, TrackDuration, TrackerId, UserId, VideoRef};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracker {
    pub id: TrackerId,

    pub video_id: VideoRef,
    pub title: String,

    pub track_start: Timestamp,
    pub track_duration: TrackDuration,

    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub created_by: UserId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Milestone {
    pub id: MilestoneId,

    pub video_id: VideoRef,
    pub title: String,

    pub track_start: Timestamp,
    pub track_duration: Timestamp,
    pub track_target: u64,

    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub created_by: UserId,
}

pub fn next_tracking_timestamp(start_at: Timestamp, duration: TrackDuration) -> Timestamp {
    let steps = start_at % duration;
    start_at + duration.stack_duration(steps)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub created_at: Timestamp,
}
