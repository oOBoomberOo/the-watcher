//! Backend for kitsune, a hololive music video tracking service.
//!

/// Commonly used types and functions within the program.
pub mod prelude {
    pub(crate) use derive_more::*;
    pub(crate) use derive_new::new;
    pub(crate) use serde::{de::DeserializeOwned, Deserialize, Serialize};
    pub(crate) use snafu::{Location, OptionExt, ResultExt, Snafu};
    pub(crate) use surrealdb::opt::IntoResource;
    pub(crate) use surrealdb::sql::Thing;
    pub(crate) use surrealdb::Surreal;
    pub(crate) use url::Url;

    pub use crate::api::{serve, App};
    pub use crate::auth::prelude::*;
    pub use crate::config::{Config, SurrealConfig};
    pub use crate::database::prelude::*;
    pub use crate::logging::{init_logger, Event, Log, Logger};
    pub use crate::time::Timestamp;
    pub use crate::tracker::prelude::*;
    pub use crate::youtube::prelude::*;

    pub use crate::InitError;
}

/// Database module
mod database;

/// Entry point for authentication
mod auth;

/// Entry point for interacting with the youtube API.
mod youtube;

/// Utility functions for dealing with time.
mod time;

/// Video tracking.
mod tracker;

/// Axum-based API endpoints.
mod api;

/// Backend config definition.
mod config;

mod logging;

use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
pub enum InitError {
    #[snafu(display("failed to load configuration"))]
    Config {
        source: envy::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("cannot bind to address: {address}"))]
    BindAddress {
        address: std::net::SocketAddr,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("error occurred while serving the app"))]
    Serve {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(transparent)]
    Tracker {
        source: tracker::TrackerInitializeError,
    },
    #[snafu(transparent)]
    Database {
        source: database::DatabaseConnectionError,
    },
    #[snafu(transparent)]
    YouTube {
        source: youtube::YouTubeConnectionError,
    },
    #[snafu(transparent)]
    Wathcer { source: tracker::WatcherSetupError },
}
