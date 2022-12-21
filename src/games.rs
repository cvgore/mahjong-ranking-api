use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    handler::Handler,
    response::{IntoResponse, Response},
    routing::{get},
    Json, Router,
};
use hyper::StatusCode;
use rand::prelude::SliceRandom;
use serde::Deserialize;
use serde_json::json;
use tower_http::compression::CompressionLayer;
use validator::Validate;

use crate::{
    app::AppError,
    db::DatabaseConnection,
    firebase, users,
    validate::{ValidatedJson, ValidatedQuery},
};

pub fn router() -> Router {
    Router::new().route(
        "/game_sessions",
        get(game_sessions_index.layer(CompressionLayer::new()))
            .post(game_sessions_create.layer(CompressionLayer::new())),
    )
}

#[derive(Clone)]
pub struct GameSessionUuid(pub String);

pub enum GameSessionUuidError {
    NotFound,
}

impl IntoResponse for GameSessionUuidError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            GameSessionUuidError::NotFound => (StatusCode::NOT_FOUND, "game session not found"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl GameSessionUuid {
    pub async fn is_started(&self, DatabaseConnection(conn): &mut DatabaseConnection) -> bool {
        sqlx::query_scalar!(
            "SELECT 1 FROM game_session_events WHERE game_session_uuid = ? AND event_type = 'start' LIMIT 1",
            self.0
        )
        .fetch_optional(conn)
        .await
        .map_or_else(|_| false, |_| true)
    }

    pub async fn is_ended(&self, DatabaseConnection(conn): &mut DatabaseConnection) -> bool {
        sqlx::query_scalar!(
            "SELECT 1 FROM game_session_events WHERE game_session_uuid = ? AND event_type = 'end' LIMIT 1",
            self.0
        )
        .fetch_optional(conn)
        .await
        .map_or_else(|_| false, |_| true)
    }

    pub async fn is_undoed(&self, DatabaseConnection(conn): &mut DatabaseConnection) -> bool {
        sqlx::query_scalar!(
            "SELECT 1 FROM game_session_events WHERE game_session_uuid = ? AND event_type = 'undo_game' LIMIT 1",
            self.0
        )
        .fetch_optional(conn)
        .await
        .map_or_else(|_| false, |_| true)
    }
}

/// unsafe because it expects that
/// it'll be used in
/// /game_session/:game_session_uuid/
#[async_trait]
impl<B> FromRequest<B> for GameSessionUuid
where
    B: Send,
{
    type Rejection = GameSessionUuidError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let uuid = {
            let uri = req.uri();

            uri
            .path()
            .split('/')
            .nth(2)
            .ok_or(GameSessionUuidError::NotFound)?
            .to_string()
        };

        if uuid.len() != crate::app::UUID_STRLEN {
            return Err(GameSessionUuidError::NotFound);
        }

        let conn = req
            .extract::<DatabaseConnection>()
            .await
            .expect("db connection is gone");

        let DatabaseConnection(mut conn) = conn;

        sqlx::query_scalar!("SELECT 1 FROM game_sessions WHERE uuid = ?", uuid)
            .fetch_optional(&mut conn)
            .await
            .map_err(|_| GameSessionUuidError::NotFound)
            .and_then(|found| match found {
                Some(_) => Ok(GameSessionUuid(uuid)),
                _ => Err(GameSessionUuidError::NotFound),
            })
    }
}

#[derive(Deserialize, Validate)]
pub struct GameSessionsCreate {
    ranking_uuid: String,
    #[validate(length(equal = 4))]
    players_uuids: Vec<String>,
    place_uuid: String,
    is_shuffled: bool,
    is_novice_friendly: bool,
    is_unranked: bool,
}

pub async fn game_sessions_create(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    ValidatedJson(input): ValidatedJson<GameSessionsCreate>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();

    let mut input = input;

    if input.is_shuffled {
        input
            .players_uuids
            .as_mut_slice()
            .shuffle(&mut rand::thread_rng());
    }

    let input = input;

    sqlx::query!(
        // sql query inserting into game sessions table
        "INSERT INTO
        game_sessions (
            uuid, creator_uuid, player1_uuid, player2_uuid, player3_uuid, player4_uuid,
            tournament_uuid, place_uuid, is_shuffled, is_novice_friendly, is_unranked,
            is_announced, is_player_certified_referee, is_league_game, ranking_uuid,
            is_tonpuu, is_too_slow, is_tenant_host, is_hidden, is_not_computed,
            is_verification_required, is_compute_skipped, created_at
        )
        VALUES (
            ?, ?, ?, ?, ?, ?,
            NULL, ?, ?, ?, ?,
            0, 0, 0, ?,
            0, 0, 0, 0, 1,
            0, 0, strftime('%s', 'now')
        )
        ",
        uuid,
        current_user.player_uuid,
        input.players_uuids[0],
        input.players_uuids[1],
        input.players_uuids[2],
        input.players_uuids[3],
        input.place_uuid,
        input.is_shuffled,
        input.is_novice_friendly,
        input.is_unranked,
        input.ranking_uuid
    )
    .execute(&mut conn)
    .await?;

    Ok(Json(json!({
        "uuid": uuid,
    })))
}

#[derive(Deserialize, Validate)]
pub struct GameSessionsIndex {
    ranking_uuid: String,
    after: Option<i64>,
}
pub async fn game_sessions_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    ValidatedQuery(input): ValidatedQuery<GameSessionsIndex>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    const PAGE_LIMIT: usize = 20;
    let mut conn = conn;
    let cursor = input.after.unwrap_or(-1).max(-1);

    let data = sqlx::query!(
        "SELECT rowid, uuid, creator_uuid, player1_uuid, player2_uuid, player3_uuid, player4_uuid,
            place_uuid, is_shuffled, is_novice_friendly, is_unranked, created_at
        FROM game_sessions 
        WHERE ranking_uuid = ? 
        AND rowid > ? 
        ORDER BY rowid
        LIMIT ?",
        input.ranking_uuid,
        cursor,
        (PAGE_LIMIT + 1) as i64
    )
    .fetch_all(&mut conn)
    .await?;

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
        "cursor": data.iter().nth(PAGE_LIMIT).map_or(None, |row| Some(row.rowid)),
    })))
}
