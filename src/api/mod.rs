use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse as _, Json, Response};
use axum::routing::*;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::config::{Config, ConfigError};
use crate::logging;
use crate::model::*;

mod error;
mod state;

pub use error::*;
pub use state::*;

pub type Result<T, E = ApiError> = std::result::Result<T, E>;
pub type App = State<state::App>;

pub async fn create_router(config: Config) -> Result<(), ConfigError> {
    let database = config.database().await?;
    let youtube = config.youtube()?;
    let state = create_app(database.clone(), youtube);

    logging::init(database);

    let app = Router::new()
        .route("/trackers", get(trackers::list))
        .route("/trackers", post(trackers::create))
        .route("/trackers/:id", get(trackers::get))
        .route("/trackers/:id", put(trackers::update))
        .route("/trackers/:id", delete(trackers::delete))
        .route("/videos/:id", get(videos::info))
        .route("/live/stats", get(live::stats))
        .route("/live/trackers", get(live::trackers))
        .with_state(state);

    let listener = config.listener().await?;

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

fn json<T: Serialize>(value: T) -> Result<Response> {
    Ok(Json(value).into_response())
}

mod live {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use futures::{future, Stream, TryStreamExt};
    use snafu::{location, Location};
    use surrealdb::{Action, Notification};
    use tracing::instrument;

    use crate::database::DatabaseError;

    use super::*;

    #[instrument(skip(app))]
    pub async fn trackers(State(app): App) -> Result<Sse<impl Stream<Item = Result<Event>>>> {
        let Ok(notifications) = app.database.select("trackers").live().into_owned().await else {
            return Err(ApiError::Internal);
        };

        let stream = notifications
            .map_ok(tracker_event)
            .map_err(into_database_error);

        let response = Sse::new(stream).keep_alive(KeepAlive::default());
        Ok(response)
    }

    fn tracker_event(notification: Notification<Tracker>) -> Event {
        let event = match notification.action {
            Action::Create => "created",
            Action::Update => "updated",
            Action::Delete => "deleted",
            _ => "unknown",
        };

        Event::default()
            .event(event)
            .json_data(notification.data)
            .unwrap()
    }

    #[instrument(skip(app))]
    pub async fn stats(State(app): App) -> Result<Sse<impl Stream<Item = Result<Event>>>> {
        let Ok(notifications) = app.database.select("stats").live().into_owned().await else {
            return Err(ApiError::Internal);
        };

        let stream = notifications
            .try_filter(|notification| future::ready(notification.action == Action::Create))
            .map_ok(notification_event)
            .map_err(into_database_error);

        let response = Sse::new(stream).keep_alive(KeepAlive::default());
        Ok(response)
    }

    fn notification_event(input: Notification<Stats>) -> Event {
        Event::default()
            .event("created")
            .json_data(input.data)
            .unwrap()
    }

    fn into_database_error(source: surrealdb::Error) -> ApiError {
        DatabaseError::DatabaseQuery {
            source,
            location: location!(),
        }
        .into()
    }
}

mod videos {
    use tracing::instrument;

    use super::*;

    #[instrument(skip(app))]
    pub async fn info(Path(id): Path<VideoId>, State(app): App) -> Result<Response> {
        let info = app.youtube().upload_info(&id).await?;
        json(info)
    }
}

mod trackers {
    use tracing::instrument;

    use crate::database::Database;

    use super::*;

    #[derive(Deserialize, Debug)]
    #[serde(default)]
    pub struct ListFilter {
        pub active: bool,
    }

    impl Default for ListFilter {
        fn default() -> Self {
            Self { active: true }
        }
    }

    #[instrument(skip(app))]
    pub async fn list(Query(filter): Query<ListFilter>, State(app): App) -> Result<Response> {
        let trackers = Tracker::trackers(filter.active, &app).await?;
        json(trackers)
    }

    #[instrument(skip(app))]
    pub async fn get(Path(id): Path<TrackerId>, State(app): App) -> Result<Response> {
        let tracker = find_tracker(id, &app).await?;
        json(tracker)
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateTracker {
        pub video_id: VideoId,
        pub track_at: Timestamp,
        pub track_duration: TrackDuration,
        #[serde(default)]
        pub track_target: Option<i64>,
    }

    #[instrument(skip(app))]
    pub async fn create(State(app): App, Json(body): Json<CreateTracker>) -> Result<Response> {
        let CreateTracker {
            video_id,
            track_at,
            track_duration,
            track_target,
        } = body;
        let tracker = Tracker::new(video_id, track_at, track_duration, track_target);

        app.schedule(tracker.clone()).await?;

        json(tracker)
    }

    #[derive(Debug, Deserialize)]
    pub struct UpdateTracker {
        pub video_id: Option<VideoId>,
        pub track_at: Option<Timestamp>,
        pub track_duration: Option<TrackDuration>,
        pub track_target: Option<i64>,
    }

    #[instrument(skip(app))]
    pub async fn update(
        State(app): App, Path(id): Path<TrackerId>, Json(update): Json<UpdateTracker>,
    ) -> Result<Response> {
        let mut tracker = find_tracker(id, &app).await?;

        let UpdateTracker {
            video_id,
            track_at,
            track_duration,
            track_target,
        } = update;

        tracker.video_id = video_id.unwrap_or(tracker.video_id);
        tracker.track_at = track_at.unwrap_or(tracker.track_at);
        tracker.track_duration = track_duration.unwrap_or(tracker.track_duration);
        tracker.track_target = track_target;

        app.update(tracker.clone()).await?;

        json(tracker)
    }

    #[instrument(skip(app))]
    pub async fn delete(Path(id): Path<TrackerId>, State(app): App) -> Result<Response> {
        app.cancel(id.clone()).await;

        let mut tracker = find_tracker(id, &app).await?;

        tracker.active = false;
        tracker.update(&app).await?;

        json(tracker)
    }

    async fn find_tracker(id: TrackerId, app: impl Into<&Database>) -> Result<Tracker> {
        Tracker::find(id.clone(), app)
            .await?
            .ok_or(ApiError::TrackerMissing { id })
    }
}
