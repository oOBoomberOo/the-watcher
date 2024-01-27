use dashmap::DashMap;
use derive_new::new;
use itertools::Itertools;
use snafu::Snafu;
use std::sync::Arc;
use tokio::select;
use tokio::time::{interval_at, Instant, Interval};
use tracing::instrument;

use super::database::{orm::tracker::UpdateTracker, Backend, BackendError};
use super::youtube::{YouTube, YouTubeError};
use crate::model::{now, Stats, Tracker, TrackerId};
use crate::service::database::orm;

#[derive(Debug, Clone, new)]
pub struct TrackerManager {
    #[new(default)]
    trackers: Arc<DashMap<TrackerId, TrackerInfo>>,
    youtube: YouTube,
    database: Backend,
}

impl TrackerManager {
    #[instrument(skip(self))]
    pub async fn update(
        &self, tracker_id: TrackerId, option: UpdateTracker,
    ) -> Result<(), TrackerError> {
        tracing::info!(tracker_id = ?tracker_id, option = ?option, "update tracker `{}`", tracker_id);
        let tracker = orm::tracker::update(tracker_id.clone(), option, &self.database).await?;

        if let Some((_id, tracker)) = self.trackers.remove(&tracker_id) {
            tracker.stop().await;
        }

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

        orm::tracker::create(tracker.clone(), &self.database).await?;
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
        let trackers = orm::tracker::all(&self.database).await?;

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
        let video_data = self.youtube.video(&tracker.video_id).await?;
        let stats = Stats::from_video_data(tracker, &video_data);

        if tracker.has_reached_target(&stats) {
            let tracker_id = tracker.id.clone();
            tracing::info!(tracker = ?tracker, stats = ?stats, "tracker `{}` has reached its target, stopping it", &tracker_id);
            self.cancel(tracker_id).await;
        }

        orm::stats::create(stats, &self.database).await?;

        Ok(())
    }

    pub(crate) async fn trackers(&self) -> Vec<TrackerId> {
        self.trackers.iter().map(|x| x.key().clone()).collect_vec()
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
    Backend { source: BackendError },
}
