mod error;
mod state;

pub use error::*;

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

pub fn create_router() {}

mod trackers {}
