use dotenvy::dotenv;

mod config;
mod database;
mod error;
mod logger;
mod model;
mod time;
mod tracker;
mod youtube;

use error::ApplicationError;

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    dotenv().ok();

    let config = config::load()?;

    let _guard = logger::init(&config)?;

    database::connect(&config.database).await?;
    let youtube = youtube::connect(&config.youtube).await;

    tracker::watcher(youtube).await
}
