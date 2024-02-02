use dashmap::DashMap;
use derive_new::new;
use itertools::Itertools;
use snafu::{OptionExt as _, Snafu};
use std::sync::Arc;
use tokio::select;
use tokio::time::{interval_at, Instant, Interval};
use tracing::instrument;

use super::youtube::{YouTube, YouTubeError};
use crate::database::Database;
use crate::database::DatabaseError;
use crate::model::{now, Tracker, TrackerId};

#[derive(Debug, Clone, new)]
pub struct TrackerManager {
    #[new(default)]
    trackers: Arc<DashMap<TrackerId, TrackerInfo>>,
    youtube: YouTube,
    database: Database,
}

impl TrackerManager {
    #[instrument(skip(self))]
    pub async fn update(&self, tracker: Tracker) -> Result<(), TrackerError> {
        let id = tracker.id.clone();
        tracing::info!(tracker_id = ?id, changes = ?tracker, "update tracker `{}`", id);

        if let Some((_id, tracker)) = self.trackers.remove(&id) {
            tracker.stop().await;
        }

        let tracker_id = tracker.id.clone();
        let tracker = tracker
            .update(&self.database)
            .await?
            .context(MissingTrackerSnafu { id })?;

        let info = self.start_task(tracker);
        self.trackers.insert(tracker_id, info);

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn schedule(&self, tracker: Tracker) -> Result<(), TrackerError> {
        tracing::info!(tracker = ?tracker, "schedule tracker `{}`", tracker.id);
        let tracker_id = tracker.id.clone();

        if let Some((_id, tracker)) = self.trackers.remove(&tracker_id) {
            tracing::info!(existing_tracker = ?tracker, new_tracker = ?tracker, "found an existing tracker with the same id, stopping it");
            tracker.stop().await;
        }

        tracker.clone().create(&self.database).await?;
        let info = self.start_task(tracker);
        self.trackers.insert(tracker_id, info);

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn cancel(&self, tracker_id: TrackerId) {
        tracing::info!("cancel tracker `{}`", tracker_id);
        if let Some((_id, tracker)) = self.trackers.remove(&tracker_id) {
            tracing::info!(tracker = ?tracker, "found the tracker `{}` and stopping id", tracker_id);
            tracker.stop().await;
        }
    }

    pub async fn fetch_all(&self) -> Result<(), TrackerError> {
        let trackers = Tracker::trackers(true, &self.database).await?;

        for tracker in trackers {
            self.schedule(tracker).await.ok();
        }

        Ok(())
    }

    pub async fn stop_all(self) {
        tracing::info!("stop all trackers");
        let tracker_ids = self.trackers.iter().map(|x| x.key().clone()).collect_vec();

        for tracker_id in tracker_ids {
            self.cancel(tracker_id).await;
        }
    }

    fn start_task(&self, tracker: Tracker) -> TrackerInfo {
        let (tx, mut message) = tokio::sync::mpsc::channel(1);
        let manager = self.clone();

        tokio::spawn(async move {
            let mut interval = get_interval(&tracker);
            tracing::info!(tracker = ?tracker, "start a background task for tracker `{}` that runs every {:?}", tracker.id, interval.period());

            loop {
                select! {
                    _ = interval.tick() => {
                        if let Err(err) = manager.run_tracker(&tracker).await {
                            tracing::error!(tracker = ?tracker, error = ?err, "error occured in tracker `{}`: {}", tracker.id, err)
                        }
                    },
                    Some(msg) = message.recv() => match msg {
                        Message::Stop => break,
                    }
                }
            }
        });

        TrackerInfo { tx }
    }

    async fn run_tracker(&self, tracker: &Tracker) -> Result<(), TrackerError> {
        let video_info = self.youtube.video(&tracker.video_id).await?;
        let stats = tracker.create_stats(video_info);

        if tracker.has_reached_target(&stats) {
            let tracker_id = tracker.id.clone();
            tracing::info!(tracker = ?tracker, stats = ?stats, "tracker `{}` has reached its target, stopping it", &tracker_id);
            self.cancel(tracker_id).await;
        }

        stats.create(&self.database).await?;

        Ok(())
    }
}

fn get_interval(tracker: &Tracker) -> Interval {
    let start = {
        let now = now();
        let timestamp = tracker.get_next_timestamp(now);
        let duration = timestamp.signed_duration_since(now.as_ref());
        Instant::now() + duration.to_std().unwrap()
    };

    let period = tracker.track_duration.duration().to_std().unwrap();
    interval_at(start, period)
}

#[derive(Debug, Clone)]
pub struct TrackerInfo {
    tx: tokio::sync::mpsc::Sender<Message>,
}

impl TrackerInfo {
    pub async fn stop(&self) {
        let _ = self.tx.send(Message::Stop).await.ok();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Message {
    Stop,
}

#[derive(Debug, Snafu)]
pub enum TrackerError {
    #[snafu(transparent)]
    YouTube { source: YouTubeError },
    #[snafu(transparent)]
    Database { source: DatabaseError },

    #[snafu(display("tracker `{}` is missing from the database", id))]
    MissingTracker { id: TrackerId },
}
