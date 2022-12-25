use axum::{extract::Path, handler::Handler, response::IntoResponse, routing::{post, get}, Json, Router};
use core::ops::Deref;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower_http::compression::CompressionLayer;
use validator::{Validate, ValidationError};

use crate::{
    app::AppError,
    db::DatabaseConnection,
    firebase,
    games::GameSessionUuid,
    users,
    validate::{ValidatedJson, ValidatedJsonBytes},
};

pub fn router() -> Router {
    Router::new()
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events",
            get(events_index),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/start",
            post(events_start),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/end",
            post(events_end),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/undo_game",
            post(events_undo_game),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/undo_last",
            post(events_undo_last),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/finish_round_by_tsumo",
            post(events_finish_round_by_tsumo),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/finish_round_by_ron",
            post(events_finish_round_by_ron),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/finish_round_by_ryuukyoku",
            post(events_finish_round_by_ryuukyoku),
        )
        .route(
            "/rankings/:ranking_uuid/game_sessions/:game_session_uuid/events/finish_round_by_chonbo",
            post(events_finish_round_by_chonbo),
        )
}

pub async fn events_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    game_session_uuid: GameSessionUuid,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let data = sqlx::query!(
        "SELECT uuid, creator_uuid, event_type, event_data, created_at
        FROM game_session_events
        WHERE game_session_uuid = ?
        ORDER BY created_at ASC",
        game_session_uuid.0,
    )
        .fetch_all(&mut conn)
        .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "uuid": row.uuid,
                "creator_uuid": row.creator_uuid,
                "event_type": row.event_type,
                "event_data": row.event_data,
                "created_at": row.created_at,
            })
        }).collect::<Vec<_>>(),
        "count": data.len(),
    })))
}

pub async fn events_start(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    game_session: GameSessionUuid,
    db: DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let GameSessionUuid(game_session_uuid) = &game_session;
    let DatabaseConnection(conn) = {
        let mut db = db;

        if game_session.is_started(&mut db).await {
            return Err(AppError::GameAlreadyStarted);
        }

        db
    };
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();
    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'start', strftime('%s', 'now')
        )
        ",
        uuid,
        *game_session_uuid,
        current_user.player_uuid
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

pub async fn events_end(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    game_session: GameSessionUuid,
    db: DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let GameSessionUuid(game_session_uuid) = &game_session;
    let DatabaseConnection(conn) = {
        let mut db = db;

        if game_session.is_ended(&mut db).await {
            return Err(AppError::GameAlreadyEnded);
        }

        db
    };
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'end', strftime('%s', 'now')
        )
        ",
        uuid,
        *game_session_uuid,
        current_user.player_uuid
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

pub async fn events_undo_game(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    game_session: GameSessionUuid,
    db: DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let GameSessionUuid(game_session_uuid) = &game_session;
    let DatabaseConnection(conn) = {
        let mut db = db;

        if game_session.is_undoed(&mut db).await {
            return Err(AppError::GameAlreadyUndone);
        }

        db
    };
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'undo_game', strftime('%s', 'now')
        )
        ",
        uuid,
        *game_session_uuid,
        current_user.player_uuid
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

pub async fn events_undo_last(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'undo_last', strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_uuid,
        current_user.player_uuid
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Serialize, Validate)]
#[validate(schema(
function = "validate_event_finish_round_tsumo_scorers_input",
skip_on_field_errors = false
))]
pub struct GameEventsFinishRoundTsumoDelta {
    scoring_player_uuid: String,
    #[allow(dead_code)]
    tile_set: Option<String>,
    // unused for now
    han: Option<i64>,
    fu: Option<i64>,
    yakuman: Option<i64>,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByTsumo {
    #[validate]
    #[validate(length(equal = 1))]
    delta: Vec<GameEventsFinishRoundTsumoDelta>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi_player_uuids: Vec<String>,
}

pub async fn events_finish_round_by_tsumo(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJsonBytes(_, bytes): ValidatedJsonBytes<GameEventsFinishRoundByTsumo>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();
    let bytes = bytes.deref();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_tsumo', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_uuid,
        current_user.player_uuid,
        bytes
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Serialize, Validate)]
#[validate(schema(
function = "validate_event_finish_round_ron_scorers_input",
skip_on_field_errors = false
))]
pub struct GameEventsFinishRoundRonDelta {
    scoring_player_uuid: String,
    losing_player_uuid: String,
    #[allow(dead_code)]
    tile_set: Option<String>,
    // unused for now
    han: Option<i64>,
    fu: Option<i64>,
    yakuman: Option<i64>,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByRon {
    #[validate]
    #[validate(length(min = 1, max = 3))]
    delta: Vec<GameEventsFinishRoundRonDelta>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi_player_uuids: Vec<String>,
}

pub async fn events_finish_round_by_ron(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJsonBytes(_, bytes): ValidatedJsonBytes<GameEventsFinishRoundByRon>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();
    let bytes = bytes.deref();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_ron', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_uuid,
        current_user.player_uuid,
        bytes
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByRyuukyoku {
    #[validate(length(min = 0, max = 4))]
    tenpai_player_uuids: Vec<String>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi_player_uuids: Vec<String>,
}

pub async fn events_finish_round_by_ryuukyoku(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJsonBytes(_, bytes): ValidatedJsonBytes<GameEventsFinishRoundByRyuukyoku>,
    DatabaseConnection(conn): DatabaseConnection,
) -> anyhow::Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();
    let bytes = bytes.deref();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_ryuukyoku', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_uuid,
        current_user.player_uuid,
        bytes
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByChonbo {
    player_uuid: String,
}

pub async fn events_finish_round_by_chonbo(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJsonBytes(_, bytes): ValidatedJsonBytes<GameEventsFinishRoundByChonbo>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = uuid::Uuid::new_v4().as_hyphenated().to_string();
    let bytes = bytes.deref();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            uuid, game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_chonbo', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_uuid,
        current_user.player_uuid,
        bytes
    )
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::CREATED)
}

fn validate_event_finish_round_ron_scorers_input(
    input: &GameEventsFinishRoundRonDelta,
) -> Result<(), ValidationError> {
    if input.tile_set.is_some() {
        Err(ValidationError::new("tile_set currently unsupported"))
    } else if input.yakuman.is_some() && input.han.is_some() && input.fu.is_some() {
        Err(ValidationError::new(
            "only yakuman or han and fu can be specified",
        ))
    } else if input.yakuman.is_some() {
        Ok(())
    } else if input.han.is_some() && input.fu.is_some() {
        Ok(())
    } else {
        Err(ValidationError::new(
            "yakuman or han and fu must be specified",
        ))
    }
}

fn validate_event_finish_round_tsumo_scorers_input(
    input: &GameEventsFinishRoundTsumoDelta,
) -> Result<(), ValidationError> {
    if input.tile_set.is_some() {
        Err(ValidationError::new("tile_set currently unsupported"))
    } else if input.yakuman.is_some() && input.han.is_some() && input.fu.is_some() {
        Err(ValidationError::new(
            "only yakuman or han and fu can be specified",
        ))
    } else if input.yakuman.is_some() {
        Ok(())
    } else if input.han.is_some() && input.fu.is_some() {
        Ok(())
    } else {
        Err(ValidationError::new(
            "yakuman or han and fu must be specified",
        ))
    }
}
