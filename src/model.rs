use chrono::Duration;
use derive_new::new;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::service::youtube::VideoId;

pub type Timestamp = chrono::DateTime<chrono::Utc>;

pub fn now() -> Timestamp {
    chrono::Utc::now()
}

pub use log::*;
pub use stats::*;
pub use tracker::*;

mod log;
mod stats;
mod tracker;
