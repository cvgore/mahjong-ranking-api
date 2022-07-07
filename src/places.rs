use axum::{Router, Json, routing::get, response::IntoResponse, handler::Handler};
use serde::{Deserialize};
use serde_json::json;
use tower_http::compression::CompressionLayer;
use validator::{Validate};

use crate::{firebase, users, db::DatabaseConnection, app::{AppError, internal_error}, validate::{ValidatedQuery}};

pub fn router() -> Router {
    Router::new().route(
        "/places",
        get(places_index.layer(CompressionLayer::new()))
    )
}

#[derive(Deserialize, Validate)]
pub struct PlacesIndex {
    // limit max length of name to 16 chars so at most little abuse can be done
    // also require at least three chars to start filtering places
    #[validate(length(min = 3, max = 16))]
    name: Option<String>
}
#[derive(Deserialize)]
struct PlacesModel {
    uuid: String,
    name: String,
    street: Option<String>,
    city: String,
    country_code: String,
    #[allow(dead_code)]
    rank: i64, // required by sqlx, but not used,
    #[allow(dead_code)]
    r#type: String, // unused right now
    created_at: i64
}
pub async fn places_index(
    _claims: firebase::FirebaseClaims,
    _current_user: users::CurrentUser,
    ValidatedQuery(input): ValidatedQuery<PlacesIndex>,
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<impl IntoResponse, AppError> {
    let mut conn = conn;

    let query = match input.name {
        Some(name) => {
            // LIKE 'name%' is more efficient than LIKE '%name%' or LIKE '%name' because
            // database can use index to speed up the query, as opposed to the latter two
            let escaped = format!("{}%", name.replace('%', "\\%").replace('?', "\\?"));
            sqlx::query_as!(
                PlacesModel,
                "SELECT * FROM places
                WHERE name LIKE ?
                ORDER BY name ASC
                LIMIT 5",
                escaped,
            ).fetch_all(&mut conn)
            .await
        },
        None => {
            sqlx::query_as!(
                PlacesModel,
                "SELECT * FROM places
                ORDER BY places.rank DESC, places.name ASC
                LIMIT 5"
            ).fetch_all(&mut conn)
            .await
        }
    };

    query
    .and_then(|data| {
        Ok(Json(json!({
            "items": data.iter().map(|row| {
                json!({
                    "uuid": row.uuid,
                    "name": row.name,
                    "street": row.street,
                    "city": row.city,
                    "country_code": row.country_code,
                    "created_at": row.created_at,
                })
            }).collect::<Vec<_>>(),
            "count": data.len(),
        })))
    })
    .map_err(internal_error)
}
