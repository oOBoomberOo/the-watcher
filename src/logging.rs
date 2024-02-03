use serde_json::Value;
use tracing::{field::Visit, level_filters::LevelFilter, Metadata, Subscriber};
use tracing_subscriber::{filter::filter_fn, layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::database::Database;

pub fn init(database: Database) {
    let subscriber = tracing_subscriber::fmt::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .pretty()
        .finish();

    let database_layer = DatabaseLayer::new(database).with_filter(filter_fn(crate_local));

    subscriber.with(database_layer).init();
}

fn crate_local(metadata: &Metadata<'_>) -> bool {
    metadata.target().starts_with("the_watcher")
}

#[derive(Debug, Clone)]
pub struct DatabaseLayer {
    tx: tokio::sync::mpsc::Sender<serde_json::Value>,
}

impl DatabaseLayer {
    pub fn new(database: Database) -> DatabaseLayer {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(err) = database.create::<Vec<()>>("logs").content(event).await {
                    eprintln!("Failed to write to database: {}", err);
                }
            }
        });

        DatabaseLayer { tx }
    }
}

impl<S: Subscriber> Layer<S> for DatabaseLayer {
    fn on_event(
        &self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let result = JsonVisitor::record(event);
        let _ = self.tx.try_send(result);
    }
}

#[derive(Debug, Default)]
struct JsonVisitor {
    buffer: serde_json::Map<String, Value>,
}

impl JsonVisitor {
    fn record(event: &tracing::Event<'_>) -> serde_json::Value {
        let mut visitor = Self::default();
        event.record(&mut visitor);
        visitor.finish()
    }

    fn finish(self) -> serde_json::Value {
        self.buffer.into()
    }
}

impl Visit for JsonVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let field = field.name().to_owned();
        let value = format!("{:?}", value);
        self.buffer.insert(field, value.into());
    }
}
