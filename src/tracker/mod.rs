use crate::error::ApplicationError;
use crate::youtube::YouTube;

mod task;

mod recorder;
mod watcher;

pub async fn watcher(youtube: YouTube) -> Result<(), ApplicationError> {
    let (state, tracker_events) = watcher::get_trackers().await?;
    watcher::manage_trackers(state, tracker_events, youtube).await;

    Ok(())
}
