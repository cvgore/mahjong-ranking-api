use std::error::Error;

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use serde_json::json;
use tracing::log::error;

#[derive(Debug)]
pub enum AppError {
    Forbidden,
    ValidationError(validator::ValidationErrors),
    AxumJsonDataRejection(axum::extract::rejection::JsonDataError),
    AxumQueryRejection(axum::extract::rejection::QueryRejection),
    AxumJsonSyntaxRejection(axum::extract::rejection::JsonSyntaxError),
    GameAlreadyStarted,
    GameAlreadyEnded,
    GameAlreadyUndone,
    Unknown,
}

pub fn internal_error<E: Error>(err: E) -> AppError {
    error!("internal error: {}", err);

    AppError::Unknown
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
            AppError::Unknown => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "internal server error",
                })),
            ),
            AppError::ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid input",
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
            AppError::AxumJsonSyntaxRejection(_) => (
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
