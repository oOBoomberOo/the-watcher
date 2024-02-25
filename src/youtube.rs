use invidious::MethodAsync::Reqwest;
use invidious::{ClientAsyncTrait, InvidiousError};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::Retry;
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
        let strategy = ExponentialBackoff::from_millis(1000).map(jitter).take(3);

        let client = self.invidious.clone();
        let video_id = video_id.to_owned();

        Retry::spawn(strategy, || {
            Self::get_stats(client.clone(), video_id.clone())
        })
        .await
    }

    async fn get_stats(
        invidious: invidious::ClientAsync,
        video_id: String,
    ) -> Result<Stats, YouTubeError> {
        let response = invidious.video(&video_id, None).await?;

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
    #[snafu(display("The video doesn't exist or is private: {message}"))]
    NotFound { message: String },

    #[snafu(display("{message}"))]
    Network { message: String },

    #[snafu(display("Cannot deserialize response from `{original}`: {error}"))]
    InvalidResponse { error: String, original: String },
}

impl From<InvidiousError> for YouTubeError {
    fn from(value: InvidiousError) -> Self {
        match value {
            InvidiousError::ApiError { message } => YouTubeError::NotFound { message },
            InvidiousError::Fetch { error } => YouTubeError::Network {
                message: error.to_string(),
            },
            InvidiousError::Message { message } => YouTubeError::Network { message },
            InvidiousError::SerdeError { error, original } => YouTubeError::InvalidResponse {
                error: error.to_string(),
                original: original.unwrap_or_default(),
            },
        }
    }
}
