use serde::Deserialize;

pub mod tracker {
    use axum::extract::Path;
    use axum::Json;
    use serde::{Deserialize, Serialize};
    use tracing::instrument;

    use crate::model::entity::Tracker;
    use crate::model::{Timestamp, TrackDuration, TrackerId, VideoRef};

    #[instrument]
    pub async fn info(Path(tracker_id): Path<TrackerId>) -> Tracker {
        todo!()
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct CreateInfo {
        pub video_id: VideoRef,

        pub title: String,
        pub uploader: String,
        pub is_premiere: bool,

        pub track_start: Timestamp,
        pub track_duration: TrackDuration,
    }

    #[instrument]
    pub async fn create_info(Path(video_id): Path<VideoRef>) -> anyhow::Result<CreateInfo> {
        todo!()
    }

    #[instrument]
    pub async fn create(Json(payload): Json<CreateInfo>) {
        todo!()
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct UpdateTracker {
        pub tracker_id: TrackerId,

        pub title: Option<String>,
        pub track_start: Option<Timestamp>,
        pub track_duration: Option<TrackDuration>,
    }

    #[instrument]
    pub async fn update(Json(payload): Json<UpdateTracker>) {
        todo!()
    }
}

pub mod milestone {
    use axum::extract::Path;
    use axum::Json;
    use serde::{Deserialize, Serialize};
    use tracing::instrument;

    use crate::model::entity::Milestone;
    use crate::model::{MilestoneId, Timestamp, TrackDuration, TrackerId, VideoRef};

    pub async fn info(Path(milestone_id): Path<MilestoneId>) -> Milestone {
        todo!()
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct CreateMilestone {
        pub video_id: VideoRef,

        pub track_start: Timestamp,
        pub track_duration: TrackDuration,
    }

    #[instrument]
    pub async fn create(Json(payload): Json<CreateMilestone>) {
        todo!()
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct UpdateMilestone {
        pub milestone_id: MilestoneId,

        pub video_id: Option<VideoRef>,
        pub track_start: Option<Timestamp>,
        pub track_duration: Option<TrackDuration>,
        pub track_target: Option<u64>,
    }
}
