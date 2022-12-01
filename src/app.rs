use axum::{
    extract::rejection::{BytesRejection, ExtensionRejection},
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use serde_json::json;
use tracing::log;

pub const NANOID_STR_LEN: usize = 21;

#[derive(Debug)]
pub enum AppError {
    InvalidBody(BytesRejection),
    Forbidden,
    InvalidJsonSyntax(serde_json::Error),
    ValidationError(validator::ValidationErrors),
    AxumJsonDataRejection(axum::extract::rejection::JsonDataError),
    AxumQueryRejection(axum::extract::rejection::QueryRejection),
    AxumJsonSyntaxRejection(axum::extract::rejection::JsonSyntaxError),
    GameAlreadyStarted,
    GameAlreadyEnded,
    GameAlreadyUndone,
    SqlError(sqlx::Error),
    Unknown(Option<Box<dyn std::error::Error>>),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppError: {:?}", self)
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::InvalidBody(err) => Some(err),
            AppError::Forbidden => None,
            AppError::InvalidJsonSyntax(err) => Some(err),
            AppError::ValidationError(err) => Some(err),
            AppError::AxumJsonDataRejection(err) => Some(err),
            AppError::AxumQueryRejection(err) => Some(err),
            AppError::AxumJsonSyntaxRejection(err) => Some(err),
            AppError::GameAlreadyStarted => None,
            AppError::GameAlreadyEnded => None,
            AppError::GameAlreadyUndone => None,
            AppError::SqlError(err) => Some(err),
            AppError::Unknown(err) => err.as_ref().map(|err| err.as_ref()),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(inner: sqlx::Error) -> Self {
        AppError::SqlError(inner)
    }
}

impl From<ExtensionRejection> for AppError {
    fn from(inner: ExtensionRejection) -> Self {
        AppError::Unknown(Some(inner.into()))
    }
}

impl From<BytesRejection> for AppError {
    fn from(inner: BytesRejection) -> Self {
        AppError::InvalidBody(inner)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(inner: serde_json::Error) -> Self {
        AppError::InvalidJsonSyntax(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Forbidden => (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "forbidden",
                })),
            ),
            AppError::SqlError(inner) => {
                log::error!("sql error: {:?}", inner);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "internal server error",
                    })),
                )
            }
            AppError::Unknown(inner) => match inner {
                Some(err) => {
                    log::error!("unknown error: {:?}", err.as_ref());

                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "internal server error",
                        })),
                    )
                }
                None => {
                    log::error!("unknown error: no details provided");

                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": "internal server error",
                        })),
                    )
                }
            },
            AppError::ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid input",
                })),
            ),
            AppError::InvalidBody(_) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "unreadable body",
                })),
            ),
            AppError::AxumQueryRejection(_) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "malformed query string",
                })),
            ),
            AppError::AxumJsonDataRejection(_) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "malformed json",
                })),
            ),
            AppError::AxumJsonSyntaxRejection(_) | AppError::InvalidJsonSyntax(_) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "malformed json",
                })),
            ),
            AppError::GameAlreadyStarted => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "game already started",
                })),
            ),
            AppError::GameAlreadyEnded => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "game already ended",
                })),
            ),
            AppError::GameAlreadyUndone => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "game already undone",
                })),
            ),
        }
        .into_response()
    }
}
