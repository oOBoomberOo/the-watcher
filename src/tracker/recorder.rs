use crate::model::{log, Record, Tracker};
use crate::time::Timestamp;
use crate::youtube::Stats;

use super::watcher::TrackerId;

pub async fn record_stats(tracker: &TrackerId, stats: Stats, timestamp: Timestamp) {
    tracing::debug!(%tracker, ?stats, "recording stats");

    if let Err(err) = Record::create(tracker, stats.views, stats.likes, timestamp).await {
        tracing::error!(%tracker, ?stats, "failed to record stats: {}", err);

        let message = format!("{err}");
        log::error(message, tracker.clone());
    }
}

pub async fn stop_tracker(tracker: &TrackerId) {
    tracing::info!(%tracker, "stopping tracker");

    if let Err(err) = Tracker::stop(tracker).await {
        tracing::error!(%tracker, "failed to stop tracker: {}", err);

        let message = format!("could not stop tracker: {err}");
        log::error(message, tracker.clone());
    }
}
