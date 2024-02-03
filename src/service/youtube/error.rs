use crate::Located;

use super::*;
use snafu::{Location, Snafu};

pub type Result<T, E = YouTubeError> = ::std::result::Result<T, E>;

#[derive(Debug, Snafu, New)]
#[snafu(visibility(pub))]
pub enum YouTubeError {
    #[snafu(display("malformed response for video `{video_id}` at {location}: {source}"))]
    InvalidVideoBody {
        video_id: VideoId,
        original: Option<String>,
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "API returned error while fetching video `{video_id}` at {location}: {message}"
    ))]
    ExternalApi {
        video_id: VideoId,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("error occurred while fetching video `{video_id}` at {location}: {message}"))]
    DuringFetch {
        video_id: VideoId,
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(transparent)]
    ParseVideoId {
        source: ParseVideoId,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("video `{video_id}` is unavailable at {location}"))]
    VideoUnavailable {
        video_id: VideoId,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("error occurred while fetching video `{video_id}` at {location}: {source}"))]
    HolodexApi {
        video_id: VideoId,
        source: holodex::errors::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

impl Located for YouTubeError {
    fn location(&self) -> Location {
        match self {
            YouTubeError::InvalidVideoBody { location, .. }
            | YouTubeError::ExternalApi { location, .. }
            | YouTubeError::DuringFetch { location, .. }
            | YouTubeError::ParseVideoId { location, .. }
            | YouTubeError::VideoUnavailable { location, .. }
            | YouTubeError::HolodexApi { location, .. } => *location,
        }
    }
}
