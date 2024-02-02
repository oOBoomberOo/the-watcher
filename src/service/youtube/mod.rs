use chrono::{DateTime, Utc};
use derivative::Derivative;
use derive_new::new as New;
use holodex::model::{VideoFull, VideoStatus};
use holodex::Client as HolodexClient;
use invidious::{video::Video as InvidiousVideo, ClientAsync as InvidiousClient, ClientAsyncTrait};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::sync::Arc;
use tracing::instrument;

use crate::model::{ParseVideoId, VideoId};

pub use error::*;

mod error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, New)]
pub struct VideoInfo {
    pub id: VideoId,
    pub views: i64,
    pub likes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, New)]
pub struct UploadInfo {
    pub id: String,
    pub title: String,
    pub is_premiere: bool,
    pub published_at: DateTime<Utc>,
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct YouTube {
    #[derivative(Debug = "ignore")]
    invidious: InvidiousClient,
    #[derivative(Debug = "ignore")]
    holodex: Arc<HolodexClient>,
}

impl YouTube {
    pub fn new(invidious: InvidiousClient, holodex: HolodexClient) -> Self {
        Self {
            invidious,
            holodex: Arc::new(holodex),
        }
    }

    #[instrument(skip(self))]
    pub async fn upload_info(&self, video_id: &VideoId) -> Result<UploadInfo> {
        let video = self.holodex_video(video_id).await?.video;

        let is_premiere = video.status == VideoStatus::Upcoming;
        let published_at = video.published_at.unwrap_or(video.available_at);

        let upload_info = UploadInfo {
            id: video.id.to_string(),
            title: video.title,
            is_premiere,
            published_at,
        };

        return Ok(upload_info);
    }

    /// Get video data from holodex and substitute with invidious data if holodex doesn't have it
    #[instrument(skip(self))]
    pub async fn video(&self, video_id: &VideoId) -> Result<VideoInfo> {
        let stats = self.invidious_video(video_id).await?;

        let views = stats.views as i64;
        let likes = stats.likes.into();

        let video_data = VideoInfo {
            id: video_id.clone(),
            likes,
            views,
        };

        Ok(video_data)
    }

    #[instrument(skip(self))]
    async fn holodex_video(&self, video_id: &VideoId) -> Result<VideoFull> {
        tracing::info!("fetch video `{}` from holodex", video_id);
        // holodex used sync API so we do it in blocking task threadpool
        let fetch_video_task = tokio::task::spawn_blocking({
            let holodex = self.holodex.clone();
            let video_id = video_id.inner().clone();
            move || holodex.video(&video_id)
        });

        let Ok(video) = fetch_video_task.await else {
            return Err(YouTubeError::VideoUnavailable {
                video_id: video_id.clone(),
            });
        };

        video.context(HolodexApiSnafu {
            video_id: video_id.clone(),
        })
    }

    #[instrument(skip(self))]
    async fn invidious_video(&self, video_id: &VideoId) -> Result<InvidiousVideo> {
        let video_id = video_id.clone();
        tracing::info!("fetch video `{}` from invidious", video_id);
        let response = self.invidious.video(video_id.as_ref(), None).await;

        use invidious::InvidiousError::*;

        let error = match response {
            Ok(video) => return Ok(video),
            Err(ApiError { message }) => YouTubeError::ExternalApi { video_id, message },
            Err(Message { message }) => YouTubeError::DuringFetch { video_id, message },
            Err(SerdeError { original, error }) => YouTubeError::InvalidVideoBody {
                video_id,
                original,
                source: error,
            },
            Err(Fetch { error }) => YouTubeError::DuringFetch {
                video_id,
                message: error.to_string(),
            },
        };

        Err(error)
    }
}
