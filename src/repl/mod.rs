use rustyline::{history::MemHistory, Editor};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

use crate::{
    config::{Config, ConfigError},
    model::{Tracker, TrackerId},
    service::{
        database::orm::tracker::UpdateTracker,
        tracker_manager::{TrackerError, TrackerManager},
    },
};

mod parse;
pub struct Repl {
    inner: Editor<(), MemHistory>,
    message: Option<String>,
}

impl Repl {
    pub fn new() -> Result<Self, ReplError> {
        let config = rustyline::Config::default();
        let inner =
            rustyline::Editor::with_history(config, MemHistory::new()).context(RustylineSnafu)?;

        let repl = Self {
            inner,
            message: None,
        };
        Ok(repl)
    }

    pub async fn prompt(&mut self) -> Action {
        let message = self
            .message
            .as_ref()
            .map(|msg| format!("  {msg}\n"))
            .unwrap_or_default();
        let prompt = format!("{}REPL> ", message);

        let Ok(input) = self.inner.readline(&prompt) else {
            return Action::Exit;
        };

        self.message = None;

        self.inner.add_history_entry(input.clone()).ok();

        match parse::parse(&input) {
            Ok(action) => action,
            Err(err) => {
                self.reply(err.to_string());
                Action::None
            }
        }
    }

    pub fn reply(&mut self, message: String) {
        if let Some(msg) = self.message.as_mut() {
            msg.push('\n');
            msg.push_str(&message);
        } else {
            self.message = Some(message);
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum Action {
    Add {
        #[serde(flatten)]
        option: UpdateTracker,
    },
    Update {
        tracker_id: TrackerId,
        #[serde(flatten)]
        option: UpdateTracker,
    },
    Remove {
        tracker_id: TrackerId,
    },
    List,
    Restart,
    Exit,
    None,
}

#[derive(Debug, Snafu)]
pub enum ReplError {
    #[snafu(transparent)]
    Config { source: ConfigError },

    #[snafu(transparent)]
    Tracker { source: TrackerError },

    #[snafu(display("failed to initialize REPL: {}", source))]
    Rustyline {
        source: rustyline::error::ReadlineError,
    },

    #[snafu(transparent)]
    Io { source: std::io::Error },

    #[snafu(display("failed to create custom terminal: {}", source))]
    ScreenCreation { source: std::io::Error },
}

pub async fn start(repl: &mut Repl) -> Result<(), ReplError> {
    tracing::info!("starting REPL");

    let _ = dotenvy::dotenv();
    let mut config = Config::new()?;

    let mut database = config.database().await?;
    let mut youtube = config.youtube()?;
    let mut manager = TrackerManager::new(youtube, database);
    manager.fetch_all().await?;

    loop {
        match repl.prompt().await {
            Action::Exit => break,
            Action::Restart => {
                repl.reply("Begin restarting REPL".to_string());

                manager.stop_all().await;

                let _ = dotenvy::dotenv();
                config = Config::new()?;
                database = config.database().await?;
                youtube = config.youtube()?;
                manager = TrackerManager::new(youtube, database);
                manager.fetch_all().await?;

                repl.reply("restarted REPL".to_string());
            }
            Action::Add { option } => {
                let UpdateTracker {
                    video_id,
                    track_at,
                    track_duration,
                    track_target,
                } = option;
                let tracker = Tracker::new(video_id, track_at, track_duration, track_target);
                let tracker_id = tracker.id.clone();

                if capture_error(repl, manager.schedule(tracker).await) {
                    repl.reply(format!("created tracker `{}`", tracker_id));
                }
            }
            Action::Remove { tracker_id } => {
                manager.cancel(tracker_id.clone()).await;
                repl.reply(format!("removed tracker `{}`", tracker_id));
            }
            Action::Update { tracker_id, option } => {
                if capture_error(repl, manager.update(tracker_id.clone(), option).await) {
                    repl.reply(format!("updated tracker `{}`", tracker_id));
                }
            }
            Action::List => {
                let trackers = manager.trackers().await;
                let trackers = trackers
                    .iter()
                    .map(|tracker| format!("  {}", tracker))
                    .collect::<Vec<_>>()
                    .join("\n");

                repl.reply(trackers);
            }
            _ => continue,
        }
    }

    Ok(())
}

fn capture_error<E: Into<ReplError>>(_repl: &mut Repl, result: Result<(), E>) -> bool {
    if let Err(err) = result {
        tracing::error!("{}", err.into());
        false
    } else {
        true
    }
}
