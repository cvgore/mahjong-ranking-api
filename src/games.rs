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
pub struct GameSessionNid(pub String);

pub enum GameSessionNidError {
    NotFound,
}

impl IntoResponse for GameSessionNidError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            GameSessionNidError::NotFound => (StatusCode::NOT_FOUND, "game session not found"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

impl GameSessionNid {
    pub async fn is_started(&self, DatabaseConnection(conn): &mut DatabaseConnection) -> bool {
        sqlx::query_scalar!(
            "SELECT 1 FROM game_session_events WHERE game_session_nid = ? AND event_type = 'start' LIMIT 1",
            self.0
        )
        .fetch_optional(conn)
        .await
        .map_or_else(|_| false, |_| true)
    }

    pub async fn is_ended(&self, DatabaseConnection(conn): &mut DatabaseConnection) -> bool {
        sqlx::query_scalar!(
            "SELECT 1 FROM game_session_events WHERE game_session_nid = ? AND event_type = 'end' LIMIT 1",
            self.0
        )
        .fetch_optional(conn)
        .await
        .map_or_else(|_| false, |_| true)
    }

    pub async fn is_undoed(&self, DatabaseConnection(conn): &mut DatabaseConnection) -> bool {
        sqlx::query_scalar!(
            "SELECT 1 FROM game_session_events WHERE game_session_nid = ? AND event_type = 'undo_game' LIMIT 1",
            self.0
        )
        .fetch_optional(conn)
        .await
        .map_or_else(|_| false, |_| true)
    }
}

/// unsafe because it expects that
/// it'll be used in
/// /game_session/:game_session_nid/
#[async_trait]
impl<B> FromRequest<B> for GameSessionNid
where
    B: Send,
{
    type Rejection = GameSessionNidError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let nid = {
            let uri = req.uri();

            uri
            .path()
            .split('/')
            .nth(2)
            .ok_or(GameSessionNidError::NotFound)?
            .to_string()
        };

        if nid.len() != crate::app::NANOID_STR_LEN {
            return Err(GameSessionNidError::NotFound);
        }

        let conn = req
            .extract::<DatabaseConnection>()
            .await
            .expect("db connection is gone");

        let DatabaseConnection(mut conn) = conn;

        sqlx::query_scalar!("SELECT 1 FROM game_sessions WHERE nid = ?", nid)
            .fetch_optional(&mut conn)
            .await
            .map_err(|_| GameSessionNidError::NotFound)
            .and_then(|found| match found {
                Some(_) => Ok(GameSessionNid(nid)),
                _ => Err(GameSessionNidError::NotFound),
            })
    }
}

#[derive(Deserialize, Validate)]
pub struct GameSessionsCreate {
    ranking_nid: String,
    #[validate(length(equal = 4))]
    players_nids: Vec<String>,
    place_nid: String,
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
    let uuid = nanoid::nanoid!();

    let mut input = input;

    if input.is_shuffled {
        input
            .players_nids
            .as_mut_slice()
            .shuffle(&mut rand::thread_rng());
    }

    let input = input;

    sqlx::query!(
        // sql query inserting into game sessions table
        "INSERT INTO
        game_sessions (
            nid, creator_nid, player1_nid, player2_nid, player3_nid, player4_nid,
            tournament_nid, place_nid, is_shuffled, is_novice_friendly, is_unranked,
            is_announced, is_player_certified_referee, is_league_game, ranking_nid,
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
        current_user.player_nid,
        input.players_nids[0],
        input.players_nids[1],
        input.players_nids[2],
        input.players_nids[3],
        input.place_nid,
        input.is_shuffled,
        input.is_novice_friendly,
        input.is_unranked,
        input.ranking_nid
    )
    .execute(&mut conn)
    .await?;

    Ok(Json(json!({
        "uuid": uuid,
    })))
}

#[derive(Deserialize, Validate)]
pub struct GameSessionsIndex {
    ranking_nid: String,
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
        "SELECT rowid, nid, creator_nid, player1_nid, player2_nid, player3_nid, player4_nid, 
            place_nid, is_shuffled, is_novice_friendly, is_unranked, created_at
        FROM game_sessions 
        WHERE ranking_nid = ? 
        AND rowid > ? 
        ORDER BY rowid
        LIMIT ?",
        input.ranking_nid,
        cursor,
        (PAGE_LIMIT + 1) as i64
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "nid": row.nid,
                "creator_nid": row.creator_nid,
                "players_nids": [row.player1_nid, row.player2_nid, row.player3_nid, row.player4_nid],
                "place_nid": row.place_nid,
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
