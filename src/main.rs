use dotenvy::dotenv;
use the_watcher::api;
use the_watcher::config::{Config, ConfigError};

#[tokio::main]
#[snafu::report]
async fn main() -> Result<(), ConfigError> {
    dotenv().ok();
    let config = Config::new()?;
    api::create_router(config).await?;
    Ok(())
}
