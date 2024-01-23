use derive_new::new;
use serde::{Deserialize, Serialize};
use snafu::Snafu;

#[derive(Debug, Snafu, Clone, Serialize, Deserialize)]
pub enum ApiError {}
