use std::result::Result;

use snafu::ResultExt;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry;

use crate::config::Config;
use crate::error::{ApplicationError, InitializeLoggerSnafu};

pub fn init(config: &Config) -> Result<WorkerGuard, ApplicationError> {
    let (file_layer, guard) = {
        let file_appender = tracing_appender::rolling::daily(&config.log_dir, "kitsune.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let layer = layer().with_ansi(false).json().with_writer(non_blocking);

        (layer, guard)
    };

    let console_layer = layer().pretty().with_writer(std::io::stdout);

    let subscriber = registry().with(console_layer).with(file_layer);
    tracing::subscriber::set_global_default(subscriber).context(InitializeLoggerSnafu)?;

    Ok(guard)
}
