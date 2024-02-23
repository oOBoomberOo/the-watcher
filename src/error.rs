use std::net::SocketAddr;

use snafu::{Location, Snafu};

use crate::database::DatabaseError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum ApplicationError {
    /// could not parse the configuration file
    ConfigLoad {
        source: envy::Error,
        #[snafu(implicit)]
        location: Location,
    },

    ConnectDatabase {
        source: DatabaseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not get active trackers from the database
    ActiveTrackers {
        source: DatabaseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not listen to tracker events
    WatchTrackers {
        source: DatabaseError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not serve the application
    WebServer {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not bind to the given address, check if it's already in use
    BindAddress {
        address: SocketAddr,
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not initialize the logger
    InitializeLogger {
        source: tracing::subscriber::SetGlobalDefaultError,
        #[snafu(implicit)]
        location: Location,
    },

    /// Could not initialize holodex
    Holodex {
        source: holodex::errors::Error,
        #[snafu(implicit)]
        location: Location,
    },
}
