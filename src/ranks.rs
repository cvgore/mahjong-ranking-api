use axum::{handler::Handler, response::{IntoResponse, Response}, routing::{get, post}, Json, Router, async_trait, extract::{Path, FromRequest, RequestParts}};
use hyper::StatusCode;
use rand::prelude::SliceRandom;
use serde::Deserialize;
use serde_json::json;
use tower_http::compression::CompressionLayer;
use validator::Validate;

use crate::{
    app::{AppError},
    db::DatabaseConnection,
    firebase, users,
};
use crate::validate::ValidatedQuery;

pub fn router() -> Router {
    Router::new()
    .route(
        "/rankings/:ranking_uuid/ranks",
        get(ranks_index),
    )
}

pub async fn ranks_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    Path(ranking_uuid): Path<String>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let data = sqlx::query!(
        r#"SELECT
            uuid, name, required_points, required_exam, color
        FROM ranks_cache ORDER BY created_at ASC"#
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "uuid": row.uuid,
                "name": row.name,
                "required_points": row.required_points,
                "required_exam": row.required_exam,
                "color": row.color,
            })
        }).collect::<Vec<_>>(),
        "count": data.len(),
    })))
}