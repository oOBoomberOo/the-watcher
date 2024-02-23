use dashmap::DashMap;
use futures::{Future, StreamExt};
use serde::Deserialize;
use snafu::ResultExt as _;
use surrealdb::Action;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::database::database;
use crate::error::{ActiveTrackersSnafu, ApplicationError, WatchTrackersSnafu};
use crate::model::{Tracker, TrackerData};
use crate::time;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Tick {
    pub tracker: String,
    pub video: String,
    pub milestone: Option<u64>,
}

impl Tick {
    pub fn exceed_milestone(&self, views: u64) -> bool {
        self.milestone.map_or(false, |milestone| views >= milestone)
    }
}

pub type TrackerId = String;

pub(super) enum Event {
    Add { tracker: Tracker },
    Update { id: TrackerId, data: TrackerData },
    Stop { id: TrackerId },
}

pub(super) type State = DashMap<TrackerId, Task>;

pub(super) async fn get_trackers() -> Result<(State, Receiver<Event>), ApplicationError> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    let state = DashMap::new();

    let active_trackers = Tracker::all_active().await.context(ActiveTrackersSnafu)?;

    for tracker in active_trackers {
        tx.send(Event::Add { tracker })
            .await
            .expect("send add event");
    }

    let stream = database()
        .select::<Vec<Tracker>>("trackers")
        .live()
        .into_owned()
        .await
        .context(WatchTrackersSnafu)?;

    tokio::spawn(async move {
        futures::pin_mut!(stream);

        while let Some(notification) = stream.next().await {
            let notification = match notification {
                Err(error) => {
                    tracing::error!(%error, "could not receive tracker event");
                    continue;
                }

                Ok(notification) => notification,
            };

            let action = notification.action;
            let tracker = notification.data;

            match action {
                Action::Update if tracker.is_stopped() => {
                    tx.send(Event::Stop { id: tracker.id })
                        .await
                        .expect("send stop event");
                }
                Action::Update => {
                    let event = Event::Update {
                        id: tracker.id,
                        data: tracker.data,
                    };

                    tx.send(event).await.expect("send update event");
                }
                Action::Create => {
                    tx.send(Event::Add { tracker })
                        .await
                        .expect("send add event");
                }
                Action::Delete => {
                    tx.send(Event::Stop { id: tracker.id })
                        .await
                        .expect("send stop event");
                }

                _ => (),
            }
        }
    });

    Ok((state, rx))
}

pub(super) async fn manage_trackers(
    state: State,
    mut trackers: Receiver<Event>,
    ticker: Sender<Tick>,
) {
    while let Some(event) = trackers.recv().await {
        match event {
            Event::Add { tracker } => add_tracker(&state, &ticker, tracker),
            Event::Update { id, data } => update_tracker(&state, &ticker, &id, data),
            Event::Stop { id } => remove_tracker(&state, &id),
        }
    }
}

fn add_tracker(state: &State, tx: &Sender<Tick>, tracker: Tracker) {
    tracing::info!(%tracker.id, "received add tracker event");

    tracing::info!(?tracker, "added tracker");
    let task = run_tracker(tracker.id.clone(), tracker.data, tx.clone());
    state.insert(tracker.id, task);
}

fn remove_tracker(state: &State, id: &TrackerId) {
    tracing::info!(%id, "received stop tracker event");

    if let Some((id, task)) = state.remove(id) {
        tracing::debug!(tracker.id = %id, "stopping tracker");
        task.stop();
    };
}

fn update_tracker(state: &State, tx: &Sender<Tick>, id: &TrackerId, data: TrackerData) {
    tracing::info!(%id, "received update tracker event");

    let Some((id, old_task)) = state.remove(id) else {
        tracing::error!(tracker.id = %id, tracker.data = ?data, "tried to update a tracker but it cannot be found");
        return;
    };

    old_task.stop();
    tracing::info!(tracker.id = %id, tracker.data = ?data, "updated tracker");

    let task = run_tracker(id.clone(), data, tx.clone());
    state.insert(id.clone(), task);
}

pub(super) struct Task {
    _handle: tokio::task::JoinHandle<()>,
    stop: tokio::sync::oneshot::Sender<()>,
}

impl Task {
    fn new(
        stop: tokio::sync::oneshot::Sender<()>,
        f: impl Future<Output = ()> + Send + 'static,
    ) -> Self {
        Self {
            _handle: tokio::spawn(f),
            stop,
        }
    }

    fn stop(self) {
        self.stop.send(()).expect("send stop signal");
    }
}

fn run_tracker(id: TrackerId, tracker: TrackerData, events: Sender<Tick>) -> Task {
    let (stop, mut signal) = tokio::sync::oneshot::channel();

    Task::new(stop, async move {
        let mut timer = time::timer(tracker.scheduled_on, tracker.interval);

        loop {
            select! {
                _ = &mut signal => {
                    tracing::info!(tracker.id = %id, "stopped tracker");
                    break;
                }

                time = timer.tick() => {
                    tracing::debug!(tracker.id = %id, timestamp = ?time, "tracker ticked");

                    let tick = Tick {
                        tracker: id.clone(),
                        video: tracker.video.clone(),
                        milestone: tracker.milestone,
                    };

                    events.send(tick).await.expect("send tick");
                }
            }
        }
    })
}
