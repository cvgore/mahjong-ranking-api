use axum::{handler::Handler, response::{IntoResponse, Response}, routing::{get, post}, Json, Router, async_trait, extract::{Path, FromRequest, RequestParts}};
use hyper::StatusCode;
use rand::prelude::SliceRandom;
use serde::Deserialize;
use serde_json::json;
use tower_http::compression::CompressionLayer;
use validator::Validate;

use crate::{
    app::{internal_error, AppError},
    db::DatabaseConnection,
    firebase, validate::ValidatedJson, users,
};

pub fn router() -> Router {
    Router::new()
    .route(
        "/players",
        get(players_index.layer(CompressionLayer::new())),
    )
}

pub async fn players_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    sqlx::query!(
        "SELECT * FROM players_cache WHERE ranking_uuid = ? ORDER BY created_at DESC LIMIT 20",
        input.ranking_uuid,
    )
    .fetch_all(&mut conn)
    .await
    .and_then(|data| {
        Ok(Json(json!({
            "items": data.iter().map(|row| {
                json!({
                    "uuid": row.uuid,
                    "creator_uuid": row.creator_uuid,
                    "players_uuids": [row.player1_uuid, row.player2_uuid, row.player3_uuid, row.player4_uuid],
                    "place_uuid": row.place_uuid,
                    "is_shuffled": row.is_shuffled,
                    "is_novice_friendly": row.is_novice_friendly,
                    "is_unranked": row.is_unranked,
                    "created_at": row.created_at,
                })
            }).collect::<Vec<_>>(),
            "count": data.len(),
            "cursor": data.last().map_or(None, |row| Some(row.created_at)),
        })))
    })
    .map_err(internal_error)
}

