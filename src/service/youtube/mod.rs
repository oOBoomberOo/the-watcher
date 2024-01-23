use derivative::Derivative;
use derive_new::new as New;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, TimeZone, Utc};
use holodex::model::VideoFull;
use holodex::Client as HolodexClient;
use invidious::{video::Video as InvidiousVideo, ClientAsync as InvidiousClient, ClientAsyncTrait};
use snafu::OptionExt;
use std::sync::Arc;
use tracing::instrument;

pub use error::*;
pub use video_id::*;

mod error;
mod video_id;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, New)]
pub struct VideoData {
    pub id: String,
    pub title: String,
    pub views: u64,
    pub likes: u32,
    pub is_premiere: bool,
    pub published_at: Option<DateTime<Utc>>,
    /// When the video is available to watch, this can be different from published_at if the video is premiere
    pub available_at: DateTime<Utc>,
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

    /// Get video data from holodex and substitute with invidious data if holodex doesn't have it
    #[instrument(skip(self))]
    pub async fn video(&self, video_id: &VideoId) -> Result<VideoData> {
        let (content, stats) =
            tokio::join!(self.holodex_video(video_id), self.invidious_video(video_id));
        let video = content.map(|content| content.video);
        let stats = stats?;

        let is_premiere = Self::is_premiere(&stats);
        let id = video.as_ref().map_or(stats.id, |x| x.id.to_string());
        let title = video.as_ref().map_or(stats.title, |x| x.title.clone());
        let views = stats.views;
        let likes = stats.likes;

        let published_time = Self::timestamp_from_unix(stats.published)?;

        let published_at = video
            .as_ref()
            .and_then(|x| x.published_at)
            .or(Some(published_time));
        let available_at = video.as_ref().map_or(published_time, |x| x.available_at);

        Ok(VideoData {
            id,
            title,
            views,
            likes,
            is_premiere,
            published_at,
            available_at,
        })
    }

    #[instrument(skip(self))]
    async fn holodex_video(&self, video_id: &VideoId) -> Option<VideoFull> {
        // holodex used sync API so we do it in blocking task threadpool
        let fetch_video_task = tokio::task::spawn_blocking({
            let holodex = self.holodex.clone();
            let video_id = video_id.inner().clone();
            move || holodex.video(&video_id)
        });

        let Ok(video) = fetch_video_task.await else {
            tracing::warn!(%video_id, "failed to fetch video from holodex");
            return None;
        };

        let Ok(video) = video else {
            tracing::warn!(%video_id, "video is not available on holodex");
            return None;
        };

        Some(video)
    }

    #[instrument(skip(self))]
    async fn invidious_video(&self, video_id: &VideoId) -> Result<InvidiousVideo> {
        let video_id = video_id.clone();
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

    fn is_premiere(video: &InvidiousVideo) -> bool {
        video.upcoming && !video.live
    }

    fn timestamp_from_unix(unix_time: u64) -> Result<DateTime<Utc>> {
        Utc.timestamp_opt(unix_time as i64, 0)
            .earliest()
            .context(ParseInvalidTimestampSnafu {
                timestamp: unix_time,
            })
    }
}
