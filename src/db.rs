use std::{str::FromStr, time::Duration};

use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    Extension,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};

use crate::{
    app::{internal_error, AppError},
    config::Config,
};

pub struct DatabaseConnection(pub sqlx::pool::PoolConnection<sqlx::Sqlite>);

#[async_trait]
impl<B: Send> FromRequest<B> for DatabaseConnection {
    type Rejection = AppError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(pool) = Extension::<SqlitePool>::from_request(req)
            .await
            .map_err(internal_error)?;

        let conn = pool.acquire()
            .await
            .map_err(internal_error)?;

        Ok(Self(conn))
    }
}

pub async fn init_db(config: &Config) -> SqlitePool {
    let opts = SqliteConnectOptions::from_str(&config.database_url)
        .expect("unparsable sqlite conn string")
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .page_size(config.database_pragma_cache_size)
        .create_if_missing(true);

    SqlitePoolOptions::new()
        .max_connections(config.database_max_conn)
        .connect_timeout(Duration::from_secs(config.database_conn_timeout))
        .connect_with(opts)
        .await
        .expect("could not connect to sqlite")
}
