use invidious::ClientAsyncTrait;
use invidious::MethodAsync::Reqwest;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt as _, ResultExt, Snafu};
use tracing::instrument;

use crate::error::{ApplicationError, HolodexSnafu};
use crate::time::Timestamp;

pub async fn connect(config: &YouTubeConfig) -> Result<YouTube, ApplicationError> {
    let holodex = holodex::Client::new(&config.holodex_api_key).context(HolodexSnafu)?;
    let invidious = invidious::ClientAsync::new(config.invidious_instance.clone(), Reqwest);
    let youtube = YouTube { holodex, invidious };

    Ok(youtube)
}

pub fn parse_video_id(text: &str) -> Result<String, ParseVideoErr> {
    // if text is not a url, return the text
    let Ok(url) = url::Url::parse(text) else {
        return Ok(text.to_string());
    };

    // if url is youtu.be, return the first path segment
    if url.host_str() == Some("youtu.be") {
        let path = url
            .path_segments()
            .context(ExpectYouTubeUrlSnafu { text })?
            .next()
            .context(MissingIdFragmentSnafu { text })?;
        return Ok(path.to_string());
    }

    // if url is youtube.com, return the v query parameter
    if url.host_str() == Some("www.youtube.com") {
        let mut query = url.query_pairs();
        let id = query
            .find_map(|(key, value)| if key == "v" { Some(value) } else { None })
            .context(MissingIdFragmentSnafu { text })?;
        return Ok(id.to_string());
    }

    // otherwise, return an error
    Err(ParseVideoErr::ExpectYouTubeUrl {
        text: text.to_string(),
    })
}

#[derive(Debug, Snafu, PartialEq)]
pub enum ParseVideoErr {
    /// text is a valid url, but it's missing the id fragment
    MissingIdFragment { text: String },

    /// text is a url, but it doesn't point to youtube
    ExpectYouTubeUrl { text: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct YouTubeConfig {
    holodex_api_key: String,
    invidious_instance: String,
}

impl Default for YouTubeConfig {
    fn default() -> Self {
        Self {
            holodex_api_key: "".to_string(),
            invidious_instance: invidious::INSTANCE.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct YouTube {
    holodex: holodex::Client,
    invidious: invidious::ClientAsync,
}

impl YouTube {
    #[instrument(skip(self))]
    pub async fn upload_info(&self, video_id: &str) -> Result<UploadInfo, YouTubeError> {
        let video_id = video_id.parse().context(InvalidVideoIdSnafu { video_id })?;
        let holodex = self.holodex.clone();

        tokio::task::spawn_blocking(move || -> Result<UploadInfo, YouTubeError> {
            let response = holodex.video(&video_id).ok().context(NotFoundSnafu {
                video_id: video_id.to_string(),
            })?;

            Ok(UploadInfo {
                title: response.video.title,
                published_at: response.video.available_at,
            })
        })
        .await
        .unwrap()
    }

    #[instrument(skip(self))]
    pub async fn stats_info(&self, video_id: &str) -> Result<Stats, YouTubeError> {
        let response = self
            .invidious
            .video(video_id, None)
            .await
            .ok()
            .context(NotFoundSnafu { video_id })?;

        Ok(Stats {
            likes: response.likes.into(),
            views: response.views,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct UploadInfo {
    pub title: String,
    pub published_at: Timestamp,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stats {
    pub views: u64,
    pub likes: u64,
}

#[derive(Debug, Snafu)]
pub enum YouTubeError {
    /// The video id is invalid
    InvalidVideoId {
        video_id: String,
        source: holodex::errors::Error,
    },

    /// The video doesn't exist or is private
    NotFound { video_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_youtube_url() {
        let result = parse_video_id("https://www.youtube.com/watch?v=12345");
        assert_eq!(result.as_deref(), Ok("12345"));
    }

    #[test]
    fn parse_youtube_url_with_other_queries() {
        let result = parse_video_id(
            "https://www.youtube.com/watch?list=some-playlist&v=12345&feature=emb_logo",
        );
        assert_eq!(result.as_deref(), Ok("12345"));
    }

    #[test]
    fn parse_youtube_short_url() {
        let result = parse_video_id("https://youtu.be/12345");
        assert_eq!(result.as_deref(), Ok("12345"));
    }

    #[test]
    fn parse_youtube_short_url_with_other_queries() {
        let result = parse_video_id("https://youtu.be/12345?t=1");
        assert_eq!(result.as_deref(), Ok("12345"));
    }

    #[test]
    fn parse_non_url_id() {
        let result = parse_video_id("12345");
        assert_eq!(result.as_deref(), Ok("12345"));
    }

    #[test]
    fn throw_error_on_invalid_url() {
        let result = parse_video_id("https://www.youtube.com/watch");
        assert_eq!(
            result,
            Err(ParseVideoErr::MissingIdFragment {
                text: "https://www.youtube.com/watch".to_string()
            })
        );
    }

    #[test]
    fn throw_error_on_non_youtube_url() {
        let result = parse_video_id("https://www.google.com");
        assert_eq!(
            result,
            Err(ParseVideoErr::ExpectYouTubeUrl {
                text: "https://www.google.com".to_string()
            })
        );
    }
}
