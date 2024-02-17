use crate::prelude::*;

pub mod prelude {
    pub use super::holodex_service::*;
    pub use super::invidious_service::*;
    pub use super::{video_id, Stats, Video, YouTube, YouTubeConnectionError, YouTubeError};
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, new)]
pub struct Video {
    pub id: Record<Video>,
    pub title: String,
    #[new(default)]
    pub created_at: Timestamp,
}

define_table! { "videos" : Video = id }

/// Create a record from raw video id part.
pub fn video_id(id: impl AsRef<str>) -> Record<Video> {
    Record::new(id.as_ref())
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub struct Stats {
    #[new(default)]
    pub id: Record<Self>,
    #[new(default)]
    pub created_at: Timestamp,
    pub tracker: Record<Tracker>,
    pub video: Record<Video>,
    pub views: u64,
    pub likes: u64,
}

define_table! { "stats" : Stats = id }

impl Stats {
    pub async fn create(
        tracker: &Tracker,
        stats: VideoStats,
        db: impl IntoDatabase,
    ) -> Result<Only<Stats>, DatabaseQueryError> {
        db.sql("CREATE stats SET tracker = $tracker, video = $video, views = $views, likes = $likes RETURN *")
            .bind(("tracker", tracker.id()))
            .bind(("video", &tracker.video))
            .bind(("views", stats.views))
            .bind(("likes", stats.likes))
            .fetch_one()
            .await
    }
}

#[derive(Debug, Clone)]
pub struct YouTube {
    pub holodex: HolodexService,
    pub invidious: InvidiousService,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VideoInfo {
    pub title: String,
    pub published_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct VideoStats {
    pub views: u64,
    pub likes: u64,
}

#[derive(Debug, Snafu)]
pub enum YouTubeError {
    ParseVideoId {
        source: holodex::errors::Error,
        #[snafu(implicit)]
        location: Location,
    },

    InvalidInfoResponse {
        source: holodex::errors::Error,
        #[snafu(implicit)]
        location: Location,
    },

    InvalidStatsResponse {
        source: invidious::InvidiousError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu)]
pub enum YouTubeConnectionError {
    Holodex {
        api_key: String,
        source: holodex::errors::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

/// YouTube video's info lookup service.
mod holodex_service {
    use holodex::model::id::VideoId;
    use holodex::Client;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    pub struct HolodexConfig {
        pub holodex_api_key: String,
    }

    #[derive(Debug, Clone, new)]
    pub struct HolodexService {
        client: Client,
    }

    impl HolodexService {
        pub fn from_config(config: &HolodexConfig) -> Result<Self, YouTubeConnectionError> {
            let api_key = &config.holodex_api_key;
            Client::new(api_key)
                .context(HolodexSnafu { api_key })
                .map(Self::new)
        }

        pub async fn get_video_info(
            &self,
            video_id: &Record<Video>,
        ) -> Result<VideoInfo, YouTubeError> {
            let video_id: VideoId = video_id.content().parse().context(ParseVideoIdSnafu)?;
            let client = self.client.clone();

            let handle = tokio::task::spawn_blocking(move || client.video(&video_id));
            let result = handle.await.unwrap().context(InvalidInfoResponseSnafu)?;

            let info = VideoInfo {
                title: result.video.title,
                published_at: result.video.available_at.into(),
            };

            Ok(info)
        }
    }
}

/// YouTube video's stats lookup service.
mod invidious_service {
    use invidious::{ClientAsync, ClientAsyncTrait as _, MethodAsync};
    use std::fmt::Debug;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    #[serde(default)]
    pub struct InvidiousConfig {
        pub invidious_instance: String,
    }

    impl Default for InvidiousConfig {
        fn default() -> Self {
            Self {
                invidious_instance: invidious::INSTANCE.to_string(),
            }
        }
    }

    #[derive(Clone, new)]
    pub struct InvidiousService {
        client: ClientAsync,
    }

    impl InvidiousService {
        pub fn from_config(config: &InvidiousConfig) -> Self {
            Self {
                client: ClientAsync::new(config.invidious_instance.clone(), MethodAsync::Reqwest),
            }
        }

        pub async fn get_video_stats(
            &self,
            video_id: &Record<Video>,
        ) -> Result<VideoStats, YouTubeError> {
            let video_id = video_id.content();

            let stats = self
                .client
                .video(&video_id, None)
                .await
                .context(InvalidStatsResponseSnafu)?;

            let stats = VideoStats {
                views: stats.views,
                likes: stats.likes.into(),
            };

            Ok(stats)
        }
    }

    impl Debug for InvidiousService {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Invidious")
                .field("client", &"ClientAsync")
                .finish()
        }
    }

    impl Default for InvidiousService {
        fn default() -> Self {
            Self::from_config(&InvidiousConfig::default())
        }
    }
}
