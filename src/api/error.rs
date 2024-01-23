use derive_new::new;
use serde::{Deserialize, Serialize};
use snafu::Snafu;

#[derive(Debug, Snafu, Clone, Serialize, Deserialize, new)]
pub enum ApiError {
    /// Unknown error
    Unknown { message: String },
}
