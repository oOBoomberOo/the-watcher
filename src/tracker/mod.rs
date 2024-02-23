use crate::error::ApplicationError;
use crate::youtube::YouTube;

use tokio::sync::mpsc::{Receiver, Sender};

mod task;

mod recorder;
mod watcher;

pub use task::Pipe;
use tokio::task::JoinHandle;
pub use watcher::Tick;

pub async fn recorder(youtube: YouTube) -> Sender<Tick> {
    let (tracker_ticked, mut tick_events) = tokio::sync::mpsc::channel::<Tick>(100);

    tokio::spawn(async move {
        while let Some(tracker) = tick_events.recv().await {
            tracing::info!(
                tracker.id = tracker.tracker,
                tracker.video,
                tracker.milestone,
                "recording stats"
            );

            let stats = match youtube.stats_info(&tracker.video).await {
                Ok(stats) => stats,
                Err(error) => {
                    tracing::error!(%error, "could not fetch video stats");
                    continue;
                }
            };

            if tracker.exceed_milestone(stats.views) {
                recorder::stop_tracker(&tracker.tracker).await;
            }

            recorder::record_stats(&tracker.tracker, stats).await;
        }
    });

    tracker_ticked
}

pub async fn watcher() -> Result<(JoinHandle<()>, Receiver<Tick>), ApplicationError> {
    let (tracker_ticked, tick_events) = tokio::sync::mpsc::channel(100);
    let (state, tracker_events) = watcher::get_trackers().await?;

    let handle = tokio::spawn(async move {
        watcher::manage_trackers(state, tracker_events, tracker_ticked).await;
    });

    Ok((handle, tick_events))
}
