use axum::{
    extract::{FromRequestParts, Query},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use serde::Deserialize;
use std::env;
use subtle::ConstantTimeEq;
use tracing::{debug, warn};

/// Optional token expected for authorization.
/// If `GESTALT_BUS_TOKEN` is unset or empty, auth is bypassed.
pub fn get_expected_token() -> Option<String> {
    env::var("GESTALT_BUS_TOKEN").ok().filter(|t| !t.is_empty())
}

/// Token Validator that compares the token in constant time.
pub struct TokenValidator;

impl TokenValidator {
    pub fn validate(expected: &str, provided: &str) -> bool {
        let expected_bytes = expected.as_bytes();
        let provided_bytes = provided.as_bytes();

        if expected_bytes.len() != provided_bytes.len() {
            return false;
        }

        expected_bytes.ct_eq(provided_bytes).into()
    }
}

/// Query parameter for WebSocket connections (`ws://...?token=xyz`)
#[derive(Deserialize)]
pub struct AuthQuery {
    token: Option<String>,
}

/// Extractor to validate the authorization token either from `Authorization: Bearer <token>`
/// header or `token` query parameter.
pub struct ValidateAuth;

impl<S> FromRequestParts<S> for ValidateAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let expected_token = match get_expected_token() {
            Some(t) => t,
            None => {
                debug!("Auth bypassed: GESTALT_BUS_TOKEN is not set");
                return Ok(Self);
            }
        };

        // Check query parameter first
        let query_token = parts
            .extract::<Query<AuthQuery>>()
            .await
            .ok()
            .and_then(|q| q.token.clone());

        let mut provided_token = query_token;

        // If not in query, check Authorization header
        if provided_token.is_none() {
            if let Some(auth_header) = parts.headers.get(axum::http::header::AUTHORIZATION) {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        provided_token = Some(token.to_string());
                    }
                }
            }
        }

        match provided_token {
            Some(token) => {
                if TokenValidator::validate(&expected_token, &token) {
                    Ok(Self)
                } else {
                    warn!("Auth failed: invalid token provided");
                    Err(StatusCode::UNAUTHORIZED.into_response())
                }
            }
            None => {
                warn!("Auth failed: missing token");
                Err(StatusCode::UNAUTHORIZED.into_response())
            }
        }
    }
}
