use axum::{async_trait, extract::{FromRequest, RequestParts}, response::{Response, IntoResponse}, Json};
use hyper::StatusCode;
use serde_json::json;

use crate::{firebase::FirebaseClaims, db::DatabaseConnection};

pub struct CurrentUser {
    pub user_uid: String,
    pub player_uuid: String
}

#[async_trait]
impl<B> FromRequest<B> for CurrentUser
where
    B: Send,
{
    type Rejection = CurrentUserError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let claims = req.extract::<FirebaseClaims>().await.expect("firebase claims are gone");
        let conn = req.extract::<DatabaseConnection>().await.expect("db connection is gone");
        let sub = claims.sub.as_str();
        let DatabaseConnection(mut conn) = conn;

        tracing::debug!("querying user with uid [{}]", sub);

        let player_uuid = sqlx::query!("SELECT player_uuid FROM user_player WHERE user_uid = ?", sub)
            .fetch_one(&mut conn)
            .await
            .and_then(|row| Ok(row.player_uuid))
            .map_err(|_| CurrentUserError::NotAssigned)?;

        tracing::debug!("queried user with uid [{}] got uuid [{}]", sub, player_uuid);

        Ok(CurrentUser {
            user_uid: claims.sub,
            player_uuid
        })
    }
}

#[derive(Debug)]
pub enum CurrentUserError {
    NotAssigned,
    #[allow(dead_code)]
    Unknown,
}

impl IntoResponse for CurrentUserError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            CurrentUserError::NotAssigned => (StatusCode::CONFLICT, "user not assigned to player"),
            CurrentUserError::Unknown => (StatusCode::INTERNAL_SERVER_ERROR, "unknown error"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
