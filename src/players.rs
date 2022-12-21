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
        "/players",
        get(players_index.layer(CompressionLayer::new())),
    )
}

#[derive(Deserialize, Validate)]
pub struct PlayersIndex {
    ranking_uuid: String,
}
pub async fn players_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    ValidatedQuery(query): ValidatedQuery<PlayersIndex>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let data = sqlx::query!(
        r#"SELECT
            uuid, usma_id, first_name, last_name, city, region, country_code,
            nickname, "is_exam_done: bool" as is_exam_done,
            "is_gdpr_agreed: bool" as is_gdpr_agreed,
            "is_guest: bool" as is_guest, "is_static: bool" as is_static
        FROM players_cache WHERE ranking_uuid = ? ORDER BY created_at DESC"#,
        query.ranking_uuid,
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "uuid": row.uuid,
                "usma_id": row.usma_id,
                "first_name": row.first_name,
                "last_name": row.last_name,
                "city": row.city,
                "region": row.region,
                "country_code": row.country_code,
                "nickname": row.nickname,
                "is_exam_done": row.is_exam_done,
                "is_gdpr_agreed": row.is_gdpr_agreed,
                "is_guest": row.is_guest,
                "is_static": row.is_static,
            })
        }).collect::<Vec<_>>(),
        "count": data.len(),
    })))
}

