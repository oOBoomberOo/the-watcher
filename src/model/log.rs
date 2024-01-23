use serde::{Deserialize, Serialize};
use std::num::NonZeroI64;

use super::*;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub struct Log {
    pub id: Uuid,
    pub created_at: Timestamp,
    pub message: LogData,
}

impl From<LogData> for Log {
    fn from(message: LogData) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: now(),
            message,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub enum LogData {
    TrackerCreated {
        tracker: Tracker,
        video_id: VideoId,
    },
    TrackerRemoved {
        tracker: Tracker,
    },
    TrackerUpdatedDuration {
        tracker_id: TrackerId,
        old_duration: TrackDuration,
        new_duration: TrackDuration,
    },
    TrackerUpdatedVideo {
        tracker_id: TrackerId,
        old_video_id: VideoId,
        new_video_id: VideoId,
    },
    TrackerCompleted {
        tracker: Tracker,
        track_target: Option<NonZeroI64>,
        completed_stats: Stats,
    },
    TrackerTicked {
        tracker_id: TrackerId,
        video_id: VideoId,
        stats: Stats,
    },
}
