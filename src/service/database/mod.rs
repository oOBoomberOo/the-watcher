use crate::model::*;
use derive_new::new;
use snafu::{OptionExt, ResultExt};
use std::ops::Deref;
use surrealdb::{engine::any::Any, Surreal};

pub use error::*;

mod error;

#[derive(Debug, Clone)]
pub struct Backend {
    database: Surreal<Any>,
}

impl Backend {
    pub async fn new(address: &str) -> Result<Self> {
        let database =
            surrealdb::engine::any::connect(address)
                .await
                .context(DatabaseConnectionSnafu {
                    url: address.to_string(),
                })?;

        Ok(Self { database })
    }
}

impl Deref for Backend {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.database
    }
}

pub mod orm {
    use super::*;

    pub mod tracker {
        use serde::{Deserialize, Serialize};

        use super::*;

        #[derive(new, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
        pub struct UpdateTracker {
            pub video_id: VideoId,
            pub track_at: Timestamp,
            pub track_duration: TrackDuration,
            pub track_target: Option<i64>,
        }

        pub async fn create(tracker: Tracker, db: Backend) -> Result<Tracker> {
            db.create(("trackers", tracker.id.to_string()))
                .content(&tracker)
                .await
                .context(DatabaseQuerySnafu)?
                .context(EmptyQuerySnafu)
        }

        pub async fn update(id: TrackerId, payload: UpdateTracker, db: Backend) -> Result<Tracker> {
            db.update(("trackers", id.to_string()))
                .content(&payload)
                .await
                .context(DatabaseQuerySnafu)?
                .context(EmptyQuerySnafu)
        }

        pub async fn stats(id: TrackerId, db: Backend) -> Result<Vec<Stats>> {
            let mut response = db
                .query("SELECT * FROM stats WHERE tracker_id = $id ORDER BY created_at DESC")
                .bind(("id", id.to_string()))
                .await
                .context(DatabaseQuerySnafu)?;

            let stats: Vec<Stats> = response.take(0).context(DatabaseDeserializeSnafu)?;

            Ok(stats)
        }
    }

    pub mod videos {
        use super::*;

        pub async fn trackers(id: VideoId, db: Backend) -> Result<Vec<Tracker>> {
            let mut response = db
                .query("SELECT * FROM trackers WHERE video_id = $id ORDER BY created_at DESC")
                .bind(("id", id.to_string()))
                .await
                .context(DatabaseQuerySnafu)?;

            let trackers: Vec<Tracker> = response.take(0).context(DatabaseDeserializeSnafu)?;

            Ok(trackers)
        }

        pub async fn stats(id: VideoId, db: Backend) -> Result<Vec<Stats>> {
            let mut response = db
                .query("SELECT * FROM stats WHERE video_id = $id ORDER BY created_at DESC")
                .bind(("id", id.to_string()))
                .await
                .context(DatabaseQuerySnafu)?;

            let stats: Vec<Stats> = response.take(0).context(DatabaseDeserializeSnafu)?;

            Ok(stats)
        }
    }

    pub mod stats {
        use super::*;

        pub async fn create(stats: Stats, db: Backend) -> Result<Stats> {
            db.create(("stats", stats.id.to_string()))
                .content(&stats)
                .await
                .context(DatabaseQuerySnafu)?
                .context(EmptyQuerySnafu)
        }
    }
}
