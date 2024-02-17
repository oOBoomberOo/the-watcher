use std::net::SocketAddr;

use jsonwebtoken::Validation;
use secrecy::SecretString;

use crate::auth::Authenticator;
use crate::database::ServerConnection;
use crate::{prelude::*, ConfigSnafu, InitError};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "host_address")]
    pub host: SocketAddr,
    #[serde(flatten)]
    pub surreal: SurrealConfig,
    #[serde(flatten)]
    pub holodex: HolodexConfig,
    #[serde(flatten)]
    pub invidious: InvidiousConfig,
}

impl Config {
    pub fn from_env() -> Result<Config, InitError> {
        envy::from_env::<Config>().context(ConfigSnafu)
    }

    pub fn youtube(&self) -> Result<YouTube, YouTubeConnectionError> {
        let holodex = HolodexService::from_config(&self.holodex)?;
        let invidious = InvidiousService::from_config(&self.invidious);

        Ok(YouTube { holodex, invidious })
    }

    pub async fn database(&self) -> Result<Database, DatabaseConnectionError> {
        self.surreal.connect().await
    }

    pub fn authenticator(&self, database: &Database) -> Authenticator {
        Authenticator {
            secret: SecretString::new(self.surreal.token.token.clone()),
            algorithm: self.surreal.token.algorithm,
            validation: Validation::new(self.surreal.token.algorithm),

            database: self.surreal.database.clone(),
            namespace: self.surreal.namespace.clone(),
            scope_name: self.surreal.token.scope.clone(),
            token_name: self.surreal.token.name.clone(),

            db: std::sync::Arc::new(database.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SurrealConfig {
    #[serde(rename = "surreal_endpoint")]
    pub endpoint: Url,
    #[serde(rename = "surreal_namespace")]
    pub namespace: String,
    #[serde(rename = "surreal_database")]
    pub database: String,
    #[serde(rename = "surreal_username")]
    pub username: String,
    #[serde(rename = "surreal_password")]
    pub password: String,

    #[serde(flatten)]
    pub token: SurrealTokenConfig,
}

impl Connection for SurrealConfig {
    async fn connect(&self) -> Result<Database, DatabaseConnectionError> {
        let connection = ServerConnection {
            address: &self.endpoint,
            namespace: &self.namespace,
            database: &self.database,
            username: &self.username,
            password: &self.password,
        };

        connection.connect().await
    }
}
