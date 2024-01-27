use crate::model::*;
use derive_new::new;
use snafu::Snafu;
use snafu::{OptionExt, ResultExt};
use std::{collections::HashMap, ops::Deref};
use surrealdb::{engine::any::Any, opt::auth::Database, Surreal};
use url::Url;

pub type Result<T, E = BackendError> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum BackendError {
    #[snafu(display("failed to query the database: {source}"))]
    DatabaseQuery { source: surrealdb::Error },
    #[snafu(display("failed to deserialize the database response: {source}"))]
    DatabaseDeserialize { source: surrealdb::Error },
    #[snafu(display("failed to parse the database response, response is empty"))]
    EmptyQuery,

    #[snafu(display("cannot connect to the database `{url}`: {source}"))]
    DatabaseConnection { url: Url, source: surrealdb::Error },
    #[snafu(display("url `{url}` is missing a namespace parameter (ns)"))]
    NoNamespace { url: Url },
    #[snafu(display("url `{url}` is missing a database parameter (db)"))]
    NoDatabase { url: Url },
}

#[derive(Debug, Clone)]
pub struct Backend {
    database: Surreal<Any>,
}

impl Backend {
    pub async fn connect(url: Url) -> Result<Self> {
        let username = url.username();
        let password = url.password().unwrap_or("");

        let queries: HashMap<String, String> = url
            .query_pairs()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();

        let namespace = queries
            .get("ns")
            .context(NoNamespaceSnafu { url: url.clone() })?;
        let database = queries
            .get("db")
            .context(NoDatabaseSnafu { url: url.clone() })?;

        let db = surrealdb::engine::any::connect(url.as_str())
            .await
            .context(DatabaseConnectionSnafu { url: url.clone() })?;

        db.signin(Database {
            username,
            password,
            namespace,
            database,
        })
        .await
        .context(DatabaseConnectionSnafu { url: url.clone() })?;

        Ok(Self { database: db })
    }
}

impl Deref for Backend {
    type Target = Surreal<Any>;

    fn deref(&self) -> &Self::Target {
        &self.database
    }
}

pub mod helper {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    pub enum SortOrder {
        #[serde(rename = "asc")]
        Ascending,
        #[serde(rename = "desc")]
        Descending,
    }

    impl SortOrder {
        pub fn to_order(&self) -> &str {
            match self {
                Self::Ascending => "ASC",
                Self::Descending => "DESC",
            }
        }
    }

    impl Default for SortOrder {
        fn default() -> Self {
            Self::Descending
        }
    }
}

pub mod orm {
    use super::*;

    pub mod tracker {
        use serde::{Deserialize, Serialize};

        use super::*;

        pub async fn all(db: &Backend) -> Result<Vec<Tracker>> {
            tracing::debug!("fetching all trackers from database");
            let mut response = db
                .query("SELECT * FROM trackers ORDER BY created_at DESC")
                .await
                .context(DatabaseQuerySnafu)?;
            response.take(0).context(DatabaseDeserializeSnafu)
        }

        #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
        pub struct Filter {
            #[serde(default)]
            pub from: usize,
            #[serde(default = "default_limit")]
            pub limit: usize,
            #[serde(default)]
            pub sort: helper::SortOrder,
        }

        fn default_limit() -> usize {
            100
        }

        pub async fn list(filter: Filter, db: &Backend) -> Result<Vec<Tracker>> {
            tracing::debug!(filter = ?filter, "fetching trackers from database");
            let mut response = db
                .query("SELECT * FROM trackers START $from LIMIT $limit ORDER BY created_at $sort")
                .bind(("from", filter.from))
                .bind(("limit", filter.limit))
                .bind(("sort", filter.sort.to_order()))
                .await
                .context(DatabaseQuerySnafu)?;
            let trackers: Vec<Tracker> = response.take(0).context(DatabaseDeserializeSnafu)?;
            Ok(trackers)
        }

        #[derive(new, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
        pub struct UpdateTracker {
            pub video_id: VideoId,
            #[serde(default = "crate::model::now")]
            pub track_at: Timestamp,
            pub track_duration: TrackDuration,
            #[serde(default)]
            pub track_target: Option<i64>,
        }

        pub async fn create(tracker: Tracker, db: &Backend) -> Result<Tracker> {
            tracing::info!(tracker = ?tracker, "inserted tracker to database");
            let mut result: Vec<Tracker> = db
                .create("trackers")
                .content(&tracker)
                .await
                .context(DatabaseQuerySnafu)?;
            result.pop().context(EmptyQuerySnafu)
        }

        pub async fn update(
            id: TrackerId, payload: UpdateTracker, db: &Backend,
        ) -> Result<Tracker> {
            tracing::debug!(tracker_id = %id, tracker = ?payload, "updated tracker in database");
            db.update(("trackers", id.to_string()))
                .content(&payload)
                .await
                .context(DatabaseQuerySnafu)?
                .context(EmptyQuerySnafu)
        }

        pub async fn stats(id: TrackerId, db: &Backend) -> Result<Vec<Stats>> {
            tracing::info!(tracker_id = %id, "fetching stats created by tracker {} from database", id);
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

        pub async fn trackers(id: VideoId, db: &Backend) -> Result<Vec<Tracker>> {
            tracing::info!(video_id = %id, "fetching trackers created for video {} from database", id);
            let mut response = db
                .query("SELECT * FROM trackers WHERE video_id = $id ORDER BY created_at DESC")
                .bind(("id", id.to_string()))
                .await
                .context(DatabaseQuerySnafu)?;

            let trackers: Vec<Tracker> = response.take(0).context(DatabaseDeserializeSnafu)?;

            Ok(trackers)
        }

        pub async fn stats(id: VideoId, db: &Backend) -> Result<Vec<Stats>> {
            tracing::info!(video_id = %id, "fetching stats created from video {} from database", id);
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

        pub async fn create(stats: Stats, db: &Backend) -> Result<Stats> {
            tracing::debug!(stats = ?stats, "inserted stats to database");
            db.create("stats")
                .content(&stats)
                .await
                .context(DatabaseQuerySnafu)?
                .pop()
                .context(EmptyQuerySnafu)
        }
    }
}
