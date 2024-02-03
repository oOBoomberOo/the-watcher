use std::net::SocketAddr;

use derive_new::new;
use invidious::MethodAsync;
use serde::Deserialize;
use snafu::{Location, ResultExt, Snafu};
use tokio::net::TcpListener;
use url::Url;

use crate::database::{Database, DatabaseError};
use crate::service::youtube::YouTube;
use crate::Located;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub surreal_database_url: Url,
    pub holodex_api_key: String,
    #[serde(default = "default::invidious_instance")]
    pub invidious_instance: String,
    pub host: SocketAddr,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        envy::from_env::<Self>().context(EnvSnafu)
    }

    pub async fn database(&self) -> Result<Database, ConfigError> {
        let database_url = self.surreal_database_url.clone();
        Database::connect(database_url).await.context(DatabaseSnafu)
    }

    pub fn holodex(&self) -> Result<holodex::Client, ConfigError> {
        holodex::Client::new(&self.holodex_api_key).context(HolodexSnafu)
    }

    pub fn invidious(&self) -> invidious::ClientAsync {
        invidious::ClientAsync::new(self.invidious_instance.clone(), MethodAsync::Reqwest)
    }

    pub fn youtube(&self) -> Result<YouTube, ConfigError> {
        let invidious = self.invidious();
        let holodex = self.holodex()?;
        Ok(YouTube::new(invidious, holodex))
    }

    pub async fn listener(&self) -> Result<TcpListener, ConfigError> {
        TcpListener::bind(self.host).await.context(ListenerSnafu)
    }
}

#[derive(Debug, Snafu, new)]
pub enum ConfigError {
    #[snafu(display("{location}: faild to connect to the database because {}", source))]
    Database {
        source: DatabaseError,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{location}: faild to load config from env: {}", source))]
    Env {
        source: envy::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{location} faild to create holodex client: {}", source))]
    Holodex {
        source: holodex::errors::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{location} faild to create listener: {}", source))]
    Listener {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
}

impl Located for ConfigError {
    fn location(&self) -> Location {
        match self {
            ConfigError::Database { location, .. }
            | ConfigError::Env { location, .. }
            | ConfigError::Holodex { location, .. }
            | ConfigError::Listener { location, .. } => *location,
        }
    }
}

mod default {
    pub fn invidious_instance() -> String {
        invidious::INSTANCE.to_string()
    }
}
