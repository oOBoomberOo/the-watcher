use chrono::{DateTime, Utc};
use derivative::Derivative;
use derive_new::new as New;
use futures::Future;
use holodex::model::{VideoFull, VideoStatus};
use holodex::Client as HolodexClient;
use invidious::video::Video as InvidiousVideo;
use invidious::{ClientAsync as InvidiousClient, ClientAsyncTrait};
use serde::{Deserialize, Serialize};
use snafu::{Location, OptionExt as _, ResultExt};
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

    #[instrument(skip(self))]
    pub async fn video_info(&self, video_id: &VideoId) -> Result<VideoInfo> {
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

    async fn holodex_video(&self, video_id: &VideoId) -> Result<VideoFull> {
        tracing::info!("fetch video `{}` from holodex", video_id);
        // holodex used sync API so we do it in blocking task threadpool
        let fetch_video_task = tokio::task::spawn_blocking({
            let video_id = video_id.clone();
            let holodex = self.holodex.clone();
            let raw_id = video_id.inner().clone();

            move || holodex.video(&raw_id).context(HolodexApiSnafu { video_id })
        });

        fetch_video_task.await.ok().context(VideoUnavailableSnafu {
            video_id: video_id.clone(),
        })?
    }

    #[track_caller]
    fn invidious_video(
        &self, video_id: &VideoId,
    ) -> impl Future<Output = Result<InvidiousVideo>> + '_ {
        let location = Location::default(); // [track_caller] does not work with async fn
        let video_id = video_id.clone();

        async move {
            tracing::info!("fetch video `{}` from invidious", video_id);
            let response = self.invidious.video(video_id.as_ref(), None).await;

            use invidious::InvidiousError::*;

            let error = match response {
                Ok(video) => return Ok(video),
                Err(ApiError { message }) => YouTubeError::ExternalApi {
                    video_id,
                    message,
                    location,
                },
                Err(Message { message }) => YouTubeError::DuringFetch {
                    video_id,
                    message,
                    location,
                },
                Err(SerdeError { original, error }) => YouTubeError::InvalidVideoBody {
                    video_id,
                    original,
                    source: error,
                    location,
                },
                Err(Fetch { error }) => YouTubeError::DuringFetch {
                    video_id,
                    message: error.to_string(),
                    location,
                },
            };

            Err(error)
        }
    }
}
