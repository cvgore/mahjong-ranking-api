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
            "/rankings",
            get(rankings_index),
        )
        .route(
            "/rankings/:ranking_uuid/list",
            get(rankings_list),
        )
}

pub async fn rankings_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let data = sqlx::query!(
        r#"SELECT
            uuid, name, created_at, archived_at
        FROM rankings_cache ORDER BY created_at DESC, archived_at DESC NULLS LAST"#
    )
        .fetch_all(&mut conn)
        .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "uuid": row.uuid,
                "name": row.name,
                "archived_at": row.archived_at,
                "created_at": row.created_at,
            })
        }).collect::<Vec<_>>(),
        "count": data.len(),
    })))
}

pub async fn rankings_list(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    Path(ranking_uuid): Path<String>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let data = sqlx::query!(
        r#"SELECT
            player_uuid, rank_uuid, rank_points, elo_points
        FROM ranking_snapshot_cache WHERE ranking_uuid = ?"#,
        ranking_uuid,
    )
        .fetch_all(&mut conn)
        .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "player_uuid": row.player_uuid,
                "rank_uuid": row.rank_uuid,
                "rank_points": row.rank_points,
                "elo_points": row.elo_points,
            })
        }).collect::<Vec<_>>(),
        "count": data.len(),
    })))
}

