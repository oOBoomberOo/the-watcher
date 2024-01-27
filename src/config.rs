use derive_new::new;
use invidious::MethodAsync;
use serde::Deserialize;
use snafu::{ResultExt, Snafu};

use crate::service::{
    database::{Backend, BackendError},
    youtube::YouTube,
};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub surreal_url: String,
    pub surreal_namespace: String,
    pub surreal_database: String,
    pub holodex_api_key: String,
    #[serde(default = "default::invidious_instance")]
    pub invidious_instance: String,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        envy::from_env::<Self>().context(EnvSnafu)
    }

    pub async fn database(&self) -> Result<Backend, ConfigError> {
        let database_url = self.surreal_url.clone();
        let database_namespace = self.surreal_namespace.clone();
        let database = self.surreal_database.clone();

        Backend::new(&database_url, &database_namespace, &database)
            .await
            .context(BackendSnafu {
                database_url,
                database_namespace,
                database,
            })
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
}

#[derive(Debug, Snafu, new)]
pub enum ConfigError {
    #[snafu(display(
        "faild to create database connection from url {}: {}",
        database_url,
        source
    ))]
    Backend {
        database_url: String,
        database_namespace: String,
        database: String,
        source: BackendError,
    },

    #[snafu(display("faild to load config from env: {}", source))]
    Env { source: envy::Error },

    #[snafu(display("faild to create holodex client: {}", source))]
    Holodex { source: holodex::errors::Error },
}

mod default {
    pub fn invidious_instance() -> String {
        invidious::INSTANCE.to_string()
    }
}
