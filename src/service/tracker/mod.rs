use crate::define_message;

#[derive(Debug)]
pub struct TrackerService;

define_message! {
    pub msg TrackerMsg for TrackerService {

    }
}
