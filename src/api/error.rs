use axum::{http::StatusCode, response::IntoResponse};
use derive_new::new;
use serde::Serialize;
use snafu::Snafu;

use crate::database::DatabaseError;

#[derive(Debug, Snafu, Serialize, new)]
#[non_exhaustive]
#[snafu(visibility(pub(super)))]
#[serde(tag = "error")]
pub enum ApiError {
    /// Unknown error
    Unknown { message: String },

    DatabaseConnection {
        #[serde(skip_serializing)]
        source: DatabaseError,
    },
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let response = (self.status_code(), axum::response::Json(self));
        response.into_response()
    }
}
