use std::sync::Arc;

use dotenvy::dotenv;
use the_watcher::prelude::*;

#[tokio::main]
#[snafu::report]
async fn main() -> Result<(), InitError> {
    dotenv().ok();
    let config = Config::from_env()?;

    let youtube = config.youtube()?;
    let database = config.database().await?;
    let logger = init_logger(database.clone());

    let auth = config.authenticator(&database);

    let manager = Arc::new(Manager::new(youtube, database.clone(), logger.clone()));
    let watcher = Watcher::new(manager.clone(), database, logger.clone());

    watcher.watch().await?;

    let app = App::new(config.host, logger, auth);

    serve(app).await?;
    manager.shutdown().await;

    Ok(())
}
