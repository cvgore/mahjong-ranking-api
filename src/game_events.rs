use axum::{response::{IntoResponse}, routing::{post}, Router, handler::Handler};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use validator::{Validate, ValidationError};

use crate::{
    app::{internal_error, AppError},
    db::DatabaseConnection,
    firebase, users, games::GameSessionUuid, validate::ValidatedJson,
};

pub fn router() -> Router {
    Router::new()
    .route(
        "/game_session/:game_session_uuid/events/start",
        post(events_start.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/end",
        post(events_end.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/undo_game",
        post(events_undo_game.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/undo_last",
        post(events_undo_last.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/finish_round_by_tsumo",
        post(events_finish_round_by_tsumo.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/finish_round_by_ron",
        post(events_finish_round_by_ron.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/finish_round_by_ryuukyoku",
        post(events_finish_round_by_ryuukyoku.layer(CompressionLayer::new()))
    )
    .route(
        "/game_session/:game_session_uuid/events/finish_round_by_chonbo",
        post(events_finish_round_by_chonbo.layer(CompressionLayer::new()))
    )
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
    
    #[derive(Serialize)]
    struct EventData {
        creator_uuid: String
    }

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'start', NULL, strftime('%s', 'now')
        )
        ",
        *game_session_uuid,
        current_user.player_uuid
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
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

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'end', NULL, strftime('%s', 'now')
        )
        ",
        *game_session_uuid,
        current_user.player_uuid
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
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

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'undo_game', NULL, strftime('%s', 'now')
        )
        ",
        *game_session_uuid,
        current_user.player_uuid
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
}

pub async fn events_undo_last(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;    

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'undo_last', NULL, strftime('%s', 'now')
        )
        ",
        game_session_uuid,
        current_user.player_uuid
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
}

#[derive(Deserialize, Serialize, Validate)]
#[validate(schema(function = "validate_event_finish_round_scorers_input", skip_on_field_errors = false))]
pub struct GameEventsFinishRoundScorers {
    scorer_player_uuid: String,
    ronned_player_uuid: String,
    #[allow(dead_code)]
    tile_set: Option<String>, // unused for now
    han: Option<i64>,
    fu: Option<i64>,
    yakuman: Option<i64>
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByTsumo {
    #[validate]
    #[validate(length(equal = 1))]
    scorers: Vec<GameEventsFinishRoundScorers>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi: Vec<String>,
}

pub async fn events_finish_round_by_tsumo(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByTsumo>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'finish_round_by_tsumo', ?, strftime('%s', 'now')
        )
        ",
        game_session_uuid,
        current_user.player_uuid,
        data
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByRon {
    #[validate]
    #[validate(length(min = 1, max = 3))]
    scorers: Vec<GameEventsFinishRoundScorers>,
    #[validate(length(min = 0, max = 4))]
    declared_riichi: Vec<String>,
}

pub async fn events_finish_round_by_ron(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByRon>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'finish_round_by_ron', ?, strftime('%s', 'now')
        )
        ",
        game_session_uuid,
        current_user.player_uuid,
        data
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
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
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByRyuukyoku>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'finish_round_by_ryuukyoku', ?, strftime('%s', 'now')
        )
        ",
        game_session_uuid,
        current_user.player_uuid,
        data
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
}

#[derive(Deserialize, Serialize, Validate)]
pub struct GameEventsFinishRoundByChonbo {
    player_uuid: String,
}

pub async fn events_finish_round_by_chonbo(
    _claims: firebase::FirebaseClaims,
    current_user: users::CurrentUser,
    GameSessionUuid(game_session_uuid): GameSessionUuid,
    ValidatedJson(input): ValidatedJson<GameEventsFinishRoundByChonbo>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;
    let data = serde_json::to_string(&input).expect("serialize-back failed but shouldn't");

    sqlx::query!(
        "INSERT INTO
        game_session_events (
            game_session_uuid, creator_uuid, event_type, event_data, created_at
        )
        VALUES (
            ?, ?, 'finish_round_by_chonbo', ?, strftime('%s', 'now')
        )
        ",
        game_session_uuid,
        current_user.player_uuid,
        data
    )
    .execute(&mut conn)
    .await
    .and_then(|_| {
        Ok(StatusCode::CREATED)
    })
    .map_err(internal_error)
}

fn validate_event_finish_round_scorers_input(input: &GameEventsFinishRoundScorers) -> Result<(), ValidationError>  {
    if input.tile_set.is_some() {
        Err(ValidationError::new("tile_set currently unsupported"))
    } else if input.yakuman.is_some() {
        Ok(())
    } else if input.han.is_some() && input.fu.is_some() {
        Ok(())
    } else {
        Err(ValidationError::new("yakuman or han and fu must be specified"))
    }
}