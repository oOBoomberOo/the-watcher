use super::*;
use snafu::Snafu;

pub type Result<T, E = YouTubeError> = ::std::result::Result<T, E>;

#[derive(Debug, Snafu, New)]
#[snafu(visibility(pub))]
pub enum YouTubeError {
    #[snafu(display("unable to parse unix timestamp: {timestamp}"))]
    ParseInvalidTimestamp { timestamp: u64 },

    #[snafu(display("malformed response for video `{video_id}`: {source}"))]
    InvalidVideoBody {
        video_id: VideoId,
        original: Option<String>,
        source: serde_json::Error,
    },

    #[snafu(display("API returned error while fetching video `{video_id}`: {message}"))]
    ExternalApi { video_id: VideoId, message: String },

    #[snafu(display("Error occurred while fetching video `{video_id}`: {message}"))]
    DuringFetch { video_id: VideoId, message: String },

    #[snafu(transparent)]
    ParseVideoId { source: ParseVideoId },

    #[snafu(display("video `{video_id}` is unavailable"))]
    VideoUnavailable { video_id: VideoId },

    #[snafu(display("error occurred while fetching video `{video_id}`: {source}"))]
    HolodexApi {
        video_id: VideoId,
        source: holodex::errors::Error,
    },
}
