#![feature(try_blocks)]
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
use tracker::Pipe;

#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    dotenv().ok();

    let config = config::load()?;

    let _guard = logger::init(&config)?;

    database::connect(&config.database).await?;
    let youtube = youtube::connect(&config.youtube).await?;

    let record_stats = tracker::recorder(youtube.clone()).await;
    let (handle, tracker_ticks) = tracker::watcher().await?;

    tracker_ticks.pipe(record_stats);

    handle.await.unwrap();

    Ok(())
}
