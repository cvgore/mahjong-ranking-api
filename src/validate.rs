use axum::{
    extract::{FromRequest, RequestParts, rejection::{JsonRejection}, Query},
    BoxError, async_trait, body::HttpBody, Json
};
use serde::{de::DeserializeOwned};
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
        let Json(value) = Json::<T>::from_request(req)
        .await
        .map_err(|err| match err {
            JsonRejection::JsonDataError(err) => AppError::AxumJsonDataRejection(err),
            JsonRejection::JsonSyntaxError(err) => AppError::AxumJsonSyntaxRejection(err),
            _ => AppError::Unknown,
        })?;
        value.validate().map_err(|err| AppError::ValidationError(err))?;

        Ok(ValidatedJson(value))
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
        value.validate().map_err(|err| AppError::ValidationError(err))?;

        Ok(ValidatedQuery(value))
    }
}
