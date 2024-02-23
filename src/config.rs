use std::net::SocketAddr;

use serde::Deserialize;
use snafu::ResultExt;

use crate::database::DatabaseConfig;
use crate::error::{ApplicationError, ConfigLoadSnafu};
use crate::youtube::YouTubeConfig;

pub fn load() -> Result<Config, ApplicationError> {
    envy::from_env().context(ConfigLoadSnafu)
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "host_address")]
    pub host: SocketAddr,
    #[serde(flatten)]
    pub database: DatabaseConfig,
    #[serde(flatten)]
    pub youtube: YouTubeConfig,

    #[serde(default = "defaults::log_dir")]
    pub log_dir: String,
}

mod defaults {
    pub fn log_dir() -> String {
        "logs".to_string()
    }
}
