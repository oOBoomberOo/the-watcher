use std::time::Duration;

use chrono::Utc;
use tracing::instrument;

pub type Timestamp = chrono::DateTime<Utc>;

pub type Interval = surrealdb::sql::Duration;

#[instrument]
pub fn timer(start: Timestamp, interval: Interval) -> tokio::time::Interval {
    let start = tokio::time::Instant::now() + duration_to_next_instant(start, interval, Utc::now());
    let period = *interval;

    let mut timer = tokio::time::interval_at(start, period);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    timer
}

/// compute the time until the next "interval instant" will occur.
/// this is used to construct [tokio::time::Interval] on an interval that has already started.
fn duration_to_next_instant(start: Timestamp, interval: Interval, now: Timestamp) -> Duration {
    if start > now {
        return (start - now)
            .to_std()
            .expect("duration is positive since start is in the future");
    }

    let period = interval.secs() as i64;
    let elapsed = (now - start).num_seconds();
    let seconds_left = elapsed % period;

    assert!(seconds_left >= 0, "seconds left must be positive");

    Duration::from_secs(seconds_left as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Duration;

    fn interval(duration: chrono::Duration) -> Interval {
        duration.to_std().unwrap().into()
    }

    #[test]
    fn interval_in_the_future() {
        let now = Utc::now();
        let scheduled = now + Duration::days(1);
        let interval = interval(Duration::hours(1));

        let result = duration_to_next_instant(scheduled, interval, now);
        assert_eq!(
            Duration::from_std(result).unwrap(),
            Duration::days(1),
            "interval in the future should return the time that it was scheduled"
        );
    }

    #[test]
    fn already_running_interval() {
        let now = Utc::now();
        let scheduled = now - Duration::days(1) + Duration::minutes(15);
        let interval = interval(Duration::hours(1));

        let result = duration_to_next_instant(scheduled, interval, now);
        assert_eq!(Duration::from_std(result).unwrap(), Duration::minutes(45), "interval that has already started should return the time until the next interval instant");
    }
}
