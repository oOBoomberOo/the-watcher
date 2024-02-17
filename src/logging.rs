use tracing_subscriber::EnvFilter;

use crate::prelude::*;

pub fn init_logger(database: Database) -> Logger {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .pretty()
        .init();

    Logger::spawn(database)
}

#[derive(Debug, Clone, Deserialize, Serialize, new)]
pub struct Log {
    #[new(default)]
    id: Record<Log>,
    #[new(default)]
    created_at: Timestamp,
    level: Level,
    user: Record<User>,
    #[serde(flatten)]
    event: Event,
}

define_table!("logs" : Log = id);
define_crud!(Log);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Level {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, Deserialize, Serialize, new)]
#[serde(tag = "action", content = "data")]
pub enum Event {
    TrackerCreated {
        tracker: Tracker,
    },
    TrackerUpdated {
        tracker: Tracker,
    },
    TrackerStopped {
        tracker: Tracker,
    },

    StatsRecorded {
        tracker_id: Record<Tracker>,
        video_id: String,
        stats_id: Record<Stats>,
    },

    SignedUp {
        username: String,
    },
    GeneratedToken {
        token: Record<RegistrationToken>,
    },
}

impl Event {
    pub fn record(self, user: &Record<User>) -> Log {
        Log::new(self.level(), user.clone(), self)
    }

    pub fn level(&self) -> Level {
        match self {
            Event::TrackerCreated { .. }
            | Event::TrackerUpdated { .. }
            | Event::TrackerStopped { .. }
            | Event::StatsRecorded { .. } => Level::User,
            Event::SignedUp { .. } | Event::GeneratedToken { .. } => Level::System,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    tx: tokio::sync::mpsc::Sender<Log>,
}

impl Logger {
    pub fn spawn(db: Database) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Log>(100);

        tokio::spawn(async move {
            while let Some(log) = rx.recv().await {
                log.create(&db).await.ok();
            }
        });

        Self { tx }
    }

    fn send(&self, user: &Record<User>, event: Event) {
        self.tx.try_send(event.record(user)).ok();
    }
}

macro_rules! log_helper {
    ($method:ident => $cons:ident ( $($name:ident : $arg:ty),* ) ) => {
        pub fn $method(&self, user: &Record<User>, $($name: $arg),*) {
            self.send(user, Event::$cons($($name),*));
        }
    };
}

impl Logger {
    log_helper!(tracker_created => new_tracker_created(tracker: Tracker));
    log_helper!(tracker_updated => new_tracker_updated(tracker: Tracker));
    log_helper!(tracker_stopped => new_tracker_stopped(tracker: Tracker));
    log_helper!(stats_recorded => new_stats_recorded(tracker_id: Record<Tracker>, video_id: String, stats_id: Record<Stats>));
    log_helper!(signed_up => new_signed_up(username: String));
    log_helper!(generated_token => new_generated_token(token: Record<RegistrationToken>));
}
