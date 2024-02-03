use chrono::Duration;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::database::*;
use crate::*;
use snafu::ResultExt as _;

define_id!("trackers", Tracker: self => &self.id);
define_id!("stats", Stats: self => &self.id);

define_model!(Tracker);
define_model!(Stats);

define_relation! {
    Tracker > trackers(active: bool) > Vec<Tracker>
        where "SELECT * FROM trackers WHERE active = $active"
}

define_relation! {
    Tracker > stats(id: TrackerId) > Vec<Stats>
        where "SELECT * FROM stats WHERE tracker_id = $id ORDER BY created_at DESC"
}

define_relation! {
    Tracker > stop(id: TrackerId) > Option<Tracker>
        where "UPDATE trackers SET active = false WHERE id = $id RETURN AFTER"
}

pub use stats::*;
pub use timestamp::*;
pub use tracker::*;
pub use video_id::*;

mod stats;
mod timestamp;
mod tracker;
mod video_id;
