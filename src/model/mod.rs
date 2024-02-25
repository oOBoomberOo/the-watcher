use query::Only;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use crate::database::{database, query, DatabaseError};
use crate::time::{Interval, Timestamp};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Tracker {
    pub id: Thing,
    pub created_at: Timestamp,
    pub stopped_at: Option<Timestamp>,
    #[serde(flatten)]
    pub data: TrackerData,
}

impl Tracker {
    pub fn is_stopped(&self) -> bool {
        self.stopped_at.is_some()
    }

    query! {
        all_active() -> Vec<Tracker> where
            "SELECT * FROM trackers WHERE !stopped_at ORDER BY created_at DESC"
    }

    query! {
        stop(id: &Thing) -> Only<Tracker> where
            "UPDATE $id SET stopped_at = time::now()"
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TrackerData {
    pub video: String,
    pub scheduled_on: Timestamp,
    pub interval: Interval,
    pub milestone: Option<u64>,
}

impl TrackerData {
    pub fn exceed_milestone(&self, views: u64) -> bool {
        self.milestone.map_or(false, |milestone| views >= milestone)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Record {
    pub id: Thing,
    pub tracker: Thing,
    pub views: u64,
    pub likes: u64,
}

impl Record {
    query! {
        create(tracker: &Thing, views: u64, likes: u64, created_at: Timestamp) -> Only<Record> where
            "CREATE records SET tracker = $tracker, views = $views, likes = $likes, created_at = $created_at"
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StaggeredRecord {
    pub repeat: u64,
    pub views: u64,
    pub likes: u64,
    pub created_at: Timestamp,
}

pub mod log {
    use super::*;

    pub fn error(message: String, tracker: Thing) {
        tokio::spawn(async move {
            database()
                .query("LET $log = (CREATE logs SET type = 'error', message = $message, created_at = time::now() RETURN *)")
                .query("LET $log_id = $log.id")
                .query("RELATE $tracker->wrote->$log_id")
                .bind(("message", message))
                .bind(("tracker", tracker))
                .await
                .expect("executed surrealql query");
        });
    }
}
