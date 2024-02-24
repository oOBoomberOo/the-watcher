use invidious::ClientAsyncTrait;
use invidious::MethodAsync::Reqwest;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt as _, Snafu};
use tracing::instrument;

use crate::time::Timestamp;

pub async fn connect(config: &YouTubeConfig) -> YouTube {
    let invidious = invidious::ClientAsync::new(config.invidious_instance.clone(), Reqwest);
    YouTube { invidious }
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
    invidious_instance: String,
}

impl Default for YouTubeConfig {
    fn default() -> Self {
        Self {
            invidious_instance: invidious::INSTANCE.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct YouTube {
    invidious: invidious::ClientAsync,
}

impl YouTube {
    #[instrument(skip(self))]
    pub async fn stats_info(&self, video_id: &str) -> Result<Stats, YouTubeError> {
        let response = self
            .invidious
            .video(video_id, None)
            .await
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
    #[snafu(display("The video doesn't exist or is private: {source}"))]
    NotFound {
        video_id: String,
        source: invidious::InvidiousError,
    },
}
