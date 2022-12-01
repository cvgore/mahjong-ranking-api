use axum::{
    async_trait,
    body::{Bytes, HttpBody},
    extract::{rejection::JsonRejection, FromRequest, Query, RequestParts},
    BoxError, Json,
};
use futures_util::TryFutureExt;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::app::AppError;

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, B> FromRequest<B> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = AppError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req).await.map_err(|err| match err {
            JsonRejection::JsonDataError(err) => AppError::AxumJsonDataRejection(err),
            JsonRejection::JsonSyntaxError(err) => AppError::AxumJsonSyntaxRejection(err),
            _ => AppError::Unknown(Some(err.into())),
        })?;

        value
            .validate()
            .map_err(|err| AppError::ValidationError(err))?;

        Ok(ValidatedJson(value))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidatedJsonBytes< T>(pub T, pub Bytes);

#[async_trait]
impl<T, B> FromRequest<B> for ValidatedJsonBytes<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = AppError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let (value, bytes) = Bytes::from_request(req)
            .map_err(|err| AppError::from(err))
            .await
            .and_then(|bytes| match serde_json::from_slice::<T>(&bytes) {
                Ok(value) => Ok((value, bytes)),
                Err(err) => Err(err.into()),
            })?;

        value
            .validate()
            .map_err(|err| AppError::ValidationError(err))?;

    
        Ok(ValidatedJsonBytes(value, bytes))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<T, B> FromRequest<B> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = AppError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request(req)
            .await
            .map_err(|err| AppError::AxumQueryRejection(err))?;
        value
            .validate()
            .map_err(|err| AppError::ValidationError(err))?;

        Ok(ValidatedQuery(value))
    }
}
