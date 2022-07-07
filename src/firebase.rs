use std::{sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
}, time::Duration, collections::HashSet};

use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    headers::{authorization::Bearer, Authorization, CacheControl, HeaderMapExt},
    response::IntoResponse,
    response::Response,
    Extension, Json, TypedHeader,
};
use futures_util::TryFutureExt;
use hashbrown::HashMap;
use hyper::StatusCode;
use hyper_tls::HttpsConnector;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use parking_lot::{RwLock};
use serde::Deserialize;
use serde_json::json;
use tracing::{log::warn, info, debug};

#[async_trait]
impl<B> FromRequest<B> for FirebaseClaims
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(req)
                .await
                .map_err(|_| AuthError::MissingToken)?;

        let firebase = Extension::<SharedFirebaseTokenService>::from_request(req)
            .await
            .expect("firebase token service is gone");
        firebase.verify_id_token(bearer.token()).await.map_err(|e| {
            warn!("tokend id verification failed: {}", e);
            AuthError::InvalidToken
        })
    }
}

#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    MissingToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "bearer token is missing"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "bearer token is invalid"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub type SharedFirebaseTokenService = Arc<FirebaseTokenService>;

pub struct FirebaseTokenService {
    project_id: String,
    decoding_keys: Arc<RwLock<HashMap<String, (u8, DecodingKey)>>>,
    keys_update_id: AtomicU8,
}

#[derive(Debug, Deserialize)]
pub struct FirebaseClaims {
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub sub: String,
}

#[derive(Debug, Deserialize)]
struct FirebaseCertKey {
    e: String,
    kid: String,
    n: String,
}

#[derive(Deserialize)]
struct FirebaseCerts {
    keys: Vec<FirebaseCertKey>,
}

impl FirebaseTokenService {
    const fn pubkey_url() -> &'static str {
        "https://www.googleapis.com/oauth2/v3/certs"
    }

    fn make_validator(&self) -> Validation {
        let mut validator = Validation::new(Algorithm::RS256);

        validator.set_audience(&[&self.project_id]);
        validator.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
        validator.validate_exp = true;
        validator.validate_nbf = false;
        validator.algorithms = vec![Algorithm::RS256];

        #[cfg(debug_assertions)] {
            if std::env::var("DEV_NOVALIDATE_IDTOKEN").is_ok() {
                validator.insecure_disable_signature_validation();
                validator.validate_exp = false;
                validator.aud = None;
                validator.iss = None;
                validator.required_spec_claims = HashSet::new();
            }
        }

        validator
    }

    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            decoding_keys: Arc::new(RwLock::new(HashMap::with_capacity(4))),
            keys_update_id: AtomicU8::new(0),
        }
    }

    /// This method should be invoked at most by one thread at a time
    pub async fn update_auth_keys(&self) -> Result<Duration, anyhow::Error> {
        info!("start updating auth keys");

        let res = hyper::Client::builder()
            .build::<_, hyper::Body>(HttpsConnector::new())
            .get(Self::pubkey_url().try_into().unwrap())
            .await?;
        let ttl = res
            .headers()
            .typed_try_get::<CacheControl>()
            .and_then(|cc| Ok(cc.map_or_else(|| None, |cc| cc.max_age())))
            .and_then(|secs| Ok(secs.unwrap_or(Duration::from_secs(60))))
            .unwrap_or_else(|e| {
                warn!("unable to parse cache control header: {}, assuming 60 seconds ttl", e);

                Duration::from_secs(60)
            });

        hyper::body::to_bytes(res.into_body())
            .map_err(|e| anyhow::Error::new(e))
            .await
            .and_then(|body| {
                serde_json::from_slice::<FirebaseCerts>(&body).map_err(|e| anyhow::Error::new(e))
            })
            .and_then(|certs| {
                let prev_update_id = self.keys_update_id.fetch_add(1, Ordering::SeqCst);
                let curr_update_id = prev_update_id.wrapping_add(1);
                for cert in certs.keys {
                    match DecodingKey::from_rsa_components(&cert.n, &cert.e) {
                        Ok(key) => {
                            debug!("got valid auth decoding key: kid={}", &cert.kid);
                            self.decoding_keys
                                .write()
                                .insert(cert.kid, (curr_update_id, key));
                        },
                        Err(err) => return Err(anyhow::Error::msg(format!(
                            "unable to construct auth decoding key from rsa components given by firebase: {}", err)
                        ))
                    }
                }
                // leave only keys from two previous updates (sliding window)
                self.decoding_keys
                    .write()
                    .retain(|_, (update_id, _)| *update_id == curr_update_id || *update_id == prev_update_id);
                
                Ok(())
            })?;

        info!("finished updating auth keys");

        Ok(ttl)
    }

    pub async fn verify_id_token(&self, id_token: &str) -> Result<FirebaseClaims, anyhow::Error> {
        decode_header(&id_token)
            .map_err(|e| anyhow::Error::msg(format!("unable to decode id token header: {}", e)))
            .and_then(|header| {
                if header.kid.is_none() {
                    Err(anyhow::Error::msg(
                        "kid field is missing in the id token header",
                    ))
                } else {
                    Ok(header.kid.unwrap())
                }
            })
            .and_then(|key_id| {
                self.decoding_keys
                    .read()
                    .get(&key_id)
                    .ok_or(anyhow::Error::msg(
                        "unknown kid from the id token header, not found in decoding keys",
                    ))
                    .cloned()
            })
            .and_then(|(_, decoding_key)| {
                decode::<FirebaseClaims>(id_token, &decoding_key, &self.make_validator())
                    .map_err(|e| anyhow::Error::msg(format!("unable to decode id token: {}", e)))
            })
            .and_then(|data| Ok(data.claims))
    }
}
