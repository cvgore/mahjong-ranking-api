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
    games::GameSessionNid,
    users,
    validate::{ValidatedJson, ValidatedJsonBytes},
};

pub fn router() -> Router {
    Router::new()
        .route(
            "/game_sessions/:game_session_nid/events",
            get(events_index.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/start",
            post(events_start.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/end",
            post(events_end.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/undo_game",
            post(events_undo_game.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/undo_last",
            post(events_undo_last.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/finish_round_by_tsumo",
            post(events_finish_round_by_tsumo.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/finish_round_by_ron",
            post(events_finish_round_by_ron.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/finish_round_by_ryuukyoku",
            post(events_finish_round_by_ryuukyoku.layer(CompressionLayer::new())),
        )
        .route(
            "/game_sessions/:game_session_nid/events/finish_round_by_chonbo",
            post(events_finish_round_by_chonbo.layer(CompressionLayer::new())),
        )
}

pub async fn events_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    Path(game_session_nid): Path<String>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let data = sqlx::query!(
        "SELECT nid, creator_nid, event_type, event_data, created_at
        FROM game_session_events
        WHERE game_session_nid = ?
        ORDER BY created_at ASC",
        game_session_nid
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(Json(json!({
        "items": data.iter().map(|row| {
            json!({
                "nid": row.nid,
                "creator_nid": row.creator_nid,
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
    game_session: GameSessionNid,
    db: DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let GameSessionNid(game_session_nid) = &game_session;
    let DatabaseConnection(conn) = {
        let mut db = db;

        if game_session.is_started(&mut db).await {
            return Err(AppError::GameAlreadyStarted);
        }

        db
    };
    let mut conn = conn;
    let uuid = nanoid::nanoid!();
    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'start', strftime('%s', 'now')
        )
        ",
        uuid,
        *game_session_nid,
        current_user.player_nid
    )
    .execute(&mut conn)
    .await?;

    Ok(StatusCode::CREATED)
}

pub async fn events_end(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    game_session: GameSessionNid,
    db: DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let GameSessionNid(game_session_nid) = &game_session;
    let DatabaseConnection(conn) = {
        let mut db = db;

        if game_session.is_ended(&mut db).await {
            return Err(AppError::GameAlreadyEnded);
        }

        db
    };
    let mut conn = conn;
    let uuid = nanoid::nanoid!();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'end', NULL, strftime('%s', 'now')
        )
        ",
        uuid,
        *game_session_nid,
        current_user.player_nid
    )
    .execute(&mut conn)
    .await?;

    Ok(StatusCode::CREATED)
}

pub async fn events_undo_game(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    game_session: GameSessionNid,
    db: DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let GameSessionNid(game_session_nid) = &game_session;
    let DatabaseConnection(conn) = {
        let mut db = db;

        if game_session.is_undoed(&mut db).await {
            return Err(AppError::GameAlreadyUndone);
        }

        db
    };
    let mut conn = conn;
    let uuid = nanoid::nanoid!();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'undo_game', strftime('%s', 'now')
        )
        ",
        uuid,
        *game_session_nid,
        current_user.player_nid
    )
    .execute(&mut conn)
    .await?;

    Ok(StatusCode::CREATED)
}

pub async fn events_undo_last(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionNid(game_session_nid): GameSessionNid,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = nanoid::nanoid!();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, created_at
        )
        VALUES (
            ?, ?, ?, 'undo_last', strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_nid,
        current_user.player_nid
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
pub struct GameEventsFinishRoundTsumoScorers {
    scorer_player_nid: String,
    #[allow(dead_code)]
    tile_set: Option<String>, // unused for now
    han: Option<i64>,
    fu: Option<i64>,
    yakuman: Option<i64>,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByTsumo {
    #[validate]
    #[validate(length(equal = 1))]
    scorers: Vec<GameEventsFinishRoundTsumoScorers>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi: Vec<String>,
}

pub async fn events_finish_round_by_tsumo(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionNid(game_session_nid): GameSessionNid,
    ValidatedJsonBytes(_, bytes): ValidatedJsonBytes<GameEventsFinishRoundByTsumo>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = nanoid::nanoid!();
    let bytes = bytes.deref();

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_tsumo', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_nid,
        current_user.player_nid,
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
pub struct GameEventsFinishRoundRonScorers {
    scorer_player_nid: String,
    ronned_player_nid: String,
    #[allow(dead_code)]
    tile_set: Option<String>, // unused for now
    han: Option<i64>,
    fu: Option<i64>,
    yakuman: Option<i64>,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByRon {
    #[validate]
    #[validate(length(min = 1, max = 3))]
    scorers: Vec<GameEventsFinishRoundRonScorers>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi: Vec<String>,
}

pub async fn events_finish_round_by_ron(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionNid(game_session_nid): GameSessionNid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByRon>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = nanoid::nanoid!();
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_ron', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_nid,
        current_user.player_nid,
        data
    )
    .execute(&mut conn)
    .await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByRyuukyoku {
    #[validate(length(min = 0, max = 4))]
    tenpai: Vec<String>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi: Vec<String>,
}

pub async fn events_finish_round_by_ryuukyoku(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionNid(game_session_nid): GameSessionNid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByRyuukyoku>,
    DatabaseConnection(conn): DatabaseConnection,
) -> anyhow::Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = nanoid::nanoid!();
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_ryuukyoku', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_nid,
        current_user.player_nid,
        data
    )
    .execute(&mut conn)
    .await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByChonbo {
    player_nid: String,
}

pub async fn events_finish_round_by_chonbo(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionNid(game_session_nid): GameSessionNid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByChonbo>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let uuid = nanoid::nanoid!();
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            nid, game_session_nid, creator_nid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, ?, 'finish_round_by_chonbo', ?, strftime('%s', 'now')
        )
        ",
        uuid,
        game_session_nid,
        current_user.player_nid,
        data
    )
    .execute(&mut conn)
    .await?;

    Ok(StatusCode::CREATED)
}

fn validate_event_finish_round_ron_scorers_input(
    input: &GameEventsFinishRoundRonScorers,
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
    input: &GameEventsFinishRoundTsumoScorers,
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
