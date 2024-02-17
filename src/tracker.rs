use std::sync::Arc;

use dashmap::DashMap;
use futures::{pin_mut, Future, StreamExt};
use surrealdb::{Action, Notification};

use crate::prelude::*;

pub mod prelude {
    pub use super::{Interval, Manager, Tracker, TrackerInitializeError, Watcher};
}

#[derive(Debug, Snafu)]
pub enum TrackerInitializeError {
    #[snafu(display("failed to fetch currently active trackers"))]
    FetchActiveTracker {
        source: DatabaseQueryError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub struct Tracker {
    #[new(default)]
    pub id: Record<Tracker>,
    #[new(default)]
    pub created_at: Timestamp,
    #[new(value = "true")]
    pub active: bool,
    #[new(default)]
    pub stopped_at: Option<Timestamp>,

    pub owner: Record<User>,
    pub video_id: String,
    pub title: String,

    pub scheduled_on: Timestamp,
    pub interval: Interval,
    pub milestone: Option<i64>,
}

define_table!("trackers" : Tracker = id);

define_relation! {
    Tracker > get(id: Record<Tracker>) > Option<Tracker>
        where "SELECT * FROM trackers WHERE id = $id LIMIT 1"
}

define_relation! {
    Tracker > disable(id: Record<Tracker>) > Option<Tracker>
        where "UPDATE trackers SET active = false WHERE id = $id RETURN *"
}

define_relation! {
    Tracker > find(active: bool) > Vec<Tracker>
        where "SELECT * FROM trackers WHERE active = $active"
}

/// An interval of time that the tracker will look for new stats, relative to the `start_at` timestamp.
///
/// This type can be converted to [chrono::Duration] and [std::time::Duration] by [Interval::to_chrono] and [Interval::to_std].
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, new, From)]
pub struct Interval(pub surrealdb::sql::Duration);

impl Interval {
    pub fn to_std(self) -> std::time::Duration {
        self.0.0
    }

    pub fn to_interval(self, start_at: Timestamp) -> tokio::time::Interval {
        let period = self.to_std();
        let start = start_at.to_instant();

        let mut interval = tokio::time::interval_at(start, period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval
    }
}

impl From<std::time::Duration> for Interval {
    fn from(value: std::time::Duration) -> Self {
        Self(value.into())
    }
}

type TrackerId = Record<Tracker>;

type QuitSignal = tokio::sync::oneshot::Receiver<Quit>;

#[derive(Debug, Clone, Copy)]
struct Quit;

#[derive(Debug)]
pub struct TrackingTask {
    tx: tokio::sync::oneshot::Sender<Quit>,
    handle: tokio::task::JoinHandle<()>,
}

impl TrackingTask {
    fn spawn<F>(f: impl FnOnce(QuitSignal) -> F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handle = tokio::task::spawn(f(rx));
        Self { tx, handle }
    }

    fn quit(self) {
        let _ = self.tx.send(Quit);
    }

    async fn shutdown(self) {
        let _ = self.tx.send(Quit);
        let _ = self.handle.await;
    }
}

/// A tracker manager service that spawn tracker tasks and manage their lifecycles.
#[derive(Debug, new)]
pub struct Manager {
    #[new(default)]
    pub trackers: DashMap<TrackerId, TrackingTask>,
    pub youtube: YouTube,
    pub database: Database,
    pub logger: Logger,
}

impl Manager {
    /// Start all currently active trackers.
    pub async fn start_currently_active(&self) -> Result<(), TrackerInitializeError> {
        let active_trackers = Tracker::find(true, &self.database)
            .await
            .context(FetchActiveTrackerSnafu)?;

        for tracker in active_trackers {
            self.schedule(tracker.clone());
        }

        Ok(())
    }

    pub async fn shutdown(&self) {
        let tracker_keys: Vec<TrackerId> = self.trackers.iter().map(|x| x.key().clone()).collect();

        for key in tracker_keys {
            if let Some((_, task)) = self.trackers.remove(&key) {
                task.shutdown().await;
            }
        }
    }

    pub async fn record(
        logger: &Logger,
        tracker: &Tracker,
        youtube: &YouTube,
        database: &Database,
    ) {
        let video_stats = match youtube.invidious.get_video_stats(&tracker.video_id).await {
            Ok(stats) => stats,
            Err(err) => {
                tracing::warn!("Failed to fetch video stats: {}", err);
                return;
            }
        };

        match Stats::create(tracker, video_stats, database).await {
            Err(err) => {
                tracing::warn!("Failed to create stats: {}", err);
            }
            Ok(Only(stats)) => {
                logger.stats_recorded(
                    &tracker.owner,
                    tracker.id.clone(),
                    tracker.video_id.clone(),
                    stats.id,
                );
            }
        }
    }

    /// Schedule a new tracker to be run.
    ///
    /// This will spawn a new infinite task and detach it from the scope.
    pub fn schedule(&self, tracker: Tracker) {
        let tracker_id = tracker.id.clone();
        let mut interval = tracker.interval.to_interval(tracker.scheduled_on);

        let database = self.database.clone();
        let youtube = self.youtube.clone();
        let logger = self.logger.clone();

        let task = TrackingTask::spawn(|mut quit| async move {
            loop {
                tokio::select! {
                    _ = interval.tick() => Self::record(&logger, &tracker, &youtube, &database).await,
                    _ = &mut quit => break,
                }
            }
        });

        self.trackers.insert(tracker_id, task);
    }

    /// Schedule the tracker to be run and quit the existing tracker if it exists.
    pub fn update(&self, tracker: Tracker) {
        if let Some((_, existing_tracker)) = self.trackers.remove(&tracker.id) {
            existing_tracker.quit();
        }

        self.schedule(tracker);
    }

    /// Stop the tracker with the given id.
    ///
    /// Note that this will not remove the tracker from the database. Refers to [Tracker::disable] for that.
    pub fn stop(&self, id: TrackerId) {
        if let Some((_, existing_tracker)) = self.trackers.remove(&id) {
            existing_tracker.quit();
        }
    }
}

/// A watcher service that watches for changes on the trackers table and updates the [Manager] accordingly.
#[derive(Debug, Clone, new)]
pub struct Watcher {
    pub manager: Arc<Manager>,
    pub database: Database,
    pub logger: Logger,
}

impl Watcher {
    /// Begin watching for changes on the database.
    pub async fn watch(self) -> Result<(), WatcherSetupError> {
        let Self {
            manager,
            database,
            logger,
        } = self;

        let stream = database
            .select(Tracker::resource())
            .live()
            .into_owned()
            .await
            .context(SubscriptionSnafu)?;

        tokio::task::spawn(async move {
            let stream = stream;

            pin_mut!(stream);

            while let Some(event) = stream.next().await {
                let Ok(event) = event else { continue };

                let Notification { action, data, .. } = event;

                match action {
                    Action::Update if !data.active => {
                        logger.tracker_stopped(&data.owner, data.clone());
                        manager.stop(data.id);
                    }
                    Action::Update => {
                        logger.tracker_updated(&data.owner, data.clone());
                        manager.update(data);
                    }
                    Action::Delete => {
                        logger.tracker_stopped(&data.owner, data.clone());
                        manager.stop(data.id);
                    }
                    Action::Create => {
                        logger.tracker_created(&data.owner, data.clone());
                        manager.schedule(data);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}

#[derive(Debug, Snafu)]
pub enum WatcherSetupError {
    /// Failed to subscribe for changes on the database.
    Subscription {
        source: surrealdb::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
