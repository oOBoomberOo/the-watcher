use axum::{http::StatusCode, response::IntoResponse};
use derive_new::new;
use serde::Serialize;
use serde_json::json;
use snafu::Snafu;

use crate::{
    database::DatabaseError,
    service::{tracker_manager::TrackerError, youtube::YouTubeError},
};

use super::{TrackerId, VideoId};

#[derive(Debug, Snafu, Serialize, new)]
#[non_exhaustive]
#[snafu(visibility(pub(super)))]
#[serde(tag = "type")]
pub enum ApiError {
    Internal,

    #[snafu(display("failed to deserialize response from the database"))]
    DatabaseDeserialize {
        message: String,
    },
    #[snafu(display("failed to query the database"))]
    DatabaseQuery {
        message: String,
    },
    #[snafu(display("expected a non-empty response from the database"))]
    EmptyQuery,
    #[snafu(display("failed to connect to the database"))]
    DatabaseInitialization,

    #[snafu(display("video '{video_id}' is not available"))]
    VideoUnavailable {
        video_id: VideoId,
    },

    #[snafu(display("video '{video_id}' exists but cannot be parsed"))]
    VideoParseError {
        video_id: VideoId,
        message: String,
    },

    #[snafu(display("tracker '{id}' is inactive and cannot be scheduled"))]
    TrackerInactive {
        id: TrackerId,
    },
    #[snafu(display("tracker '{id}' is missing from the database"))]
    TrackerMissing {
        id: TrackerId,
    },
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::VideoUnavailable { .. } => StatusCode::NOT_FOUND,
            Self::EmptyQuery { .. } => StatusCode::NOT_FOUND,
            Self::VideoParseError { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TrackerInactive { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TrackerMissing { .. } => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let response = (
            self.status_code(),
            axum::response::Json(json!({
                "message": self.to_string(),
                "error": self
            })),
        );

        response.into_response()
    }
}

impl From<DatabaseError> for ApiError {
    fn from(value: DatabaseError) -> Self {
        match value {
            DatabaseError::DatabaseQuery { source, .. } => ApiError::DatabaseQuery {
                message: source.to_string(),
            },
            DatabaseError::DatabaseDeserialize { source, .. } => ApiError::DatabaseDeserialize {
                message: source.to_string(),
            },
            DatabaseError::EmptyQuery { .. } => ApiError::EmptyQuery,
            DatabaseError::DatabaseConnection { .. }
            | DatabaseError::NoNamespace { .. }
            | DatabaseError::NoDatabase { .. } => ApiError::DatabaseInitialization,
        }
    }
}

impl From<YouTubeError> for ApiError {
    fn from(value: YouTubeError) -> Self {
        match value {
            YouTubeError::ExternalApi { .. } => ApiError::Internal,
            YouTubeError::DuringFetch { .. } => ApiError::Internal,
            YouTubeError::ParseVideoId { .. } => ApiError::Internal,
            YouTubeError::HolodexApi { video_id, .. } => ApiError::VideoUnavailable { video_id },
            YouTubeError::VideoUnavailable { video_id, .. } => {
                ApiError::VideoUnavailable { video_id }
            }
            YouTubeError::InvalidVideoBody {
                video_id, source, ..
            } => ApiError::VideoParseError {
                video_id,
                message: source.to_string(),
            },
        }
    }
}

impl From<TrackerError> for ApiError {
    fn from(value: TrackerError) -> Self {
        match value {
            TrackerError::Database { source, .. } => source.into(),
            TrackerError::YouTube { source, .. } => source.into(),
            TrackerError::InactiveTracker { id } => ApiError::TrackerInactive { id },
            TrackerError::MissingTracker { id } => ApiError::TrackerMissing { id },
        }
    }
}
