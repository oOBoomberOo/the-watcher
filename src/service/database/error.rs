use snafu::Snafu;

pub type Result<T, E = BackendError> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum BackendError {
    #[snafu(display(
        "Failed to connect to the database `{url}` [{namespace}/{database}]: {source}"
    ))]
    DatabaseConnection {
        url: String,
        namespace: String,
        database: String,
        source: surrealdb::Error,
    },
    #[snafu(display("Failed to query the database: {source}"))]
    DatabaseQuery { source: surrealdb::Error },
    #[snafu(display("Failed to deserialize the database response: {source}"))]
    DatabaseDeserialize { source: surrealdb::Error },
    #[snafu(display("Failed to parse the database response, response is empty"))]
    EmptyQuery,
}
