use query::Only;
use serde::{Deserialize, Serialize};

use crate::database::query;
use crate::time::{Interval, Timestamp};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Tracker {
    pub id: String,
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
        search(text: &str) -> Vec<Tracker> where
            "SELECT *, search::highlight('<b>', '</b>', 1) as title FROM trackers WHERE $text = '' OR title @1@ $text OR video CONTAINS $text ORDER BY created_at DESC"
    }

    query! {
        by_id(id: &str) -> Only<Tracker> where
            "SELECT * FROM trackers WHERE id = $id"
    }

    query! {
        create(data: TrackerData) -> Only<Tracker> where
            "CREATE trackers CONTENTS $data"
    }

    query! {
        update(id: &str, data: TrackerData) -> Only<Tracker> where
            "UPDATE $id MERGE $data"
    }

    query! {
        stop(id: &str) -> Only<Tracker> where
            "UPDATE $id SET stopped_at = time::now()"
    }

    query! {
        records(tracker: &str) -> Vec<StaggeredRecord> where
            r#"
            SELECT
                count() as repeat,
                views,
                array::last(likes),
                array::last(created_at)
            FROM
                records
            WHERE
                tracker = $tracker
            GROUP BY
                views
            ORDER BY
                created_at DESC
            "#
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TrackerData {
    pub video: String,
    pub scheduled_on: Timestamp,
    pub interval: Interval,
    pub milestone: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Record {
    pub id: String,
    pub tracker: String,
    pub views: u64,
    pub likes: u64,
}

impl Record {
    query! {
        create(tracker: &str, views: u64, likes: u64) -> Only<Record> where
            "CREATE records SET tracker = $tracker, views = $views, likes = $likes"
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StaggeredRecord {
    pub repeat: u64,
    pub views: u64,
    pub likes: u64,
    pub created_at: Timestamp,
}
