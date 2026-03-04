use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{
  HeaderMap, HeaderValue, StatusCode,
  header::{AUTHORIZATION, CONTENT_TYPE},
};
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ApiError;
use super::utils::error_response;
use crate::models;

const BEARER_PREFIX: &str = "Bearer ";

#[derive(Clone)]
pub struct AuthContext {
  encoding_key: EncodingKey,
  decoding_key: DecodingKey,
  access_ttl_seconds: i64,
  refresh_ttl_seconds: i64,
}

#[derive(Debug, Clone)]
pub(super) struct AuthenticatedUserId(pub String);

#[derive(Debug, Clone)]
pub(super) struct IssuedTokenPair {
  pub access_token: String,
  pub refresh_token: String,
  pub access_expires_in: i64,
  pub refresh_expires_in: i64,
}

#[derive(Debug, Clone)]
pub(super) struct IssuedAccessToken {
  pub access_token: String,
  pub access_expires_in: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
  Access,
  Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtClaims {
  sub: String,
  exp: i64,
  iat: i64,
  token_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AccessTokenValidationError {
  Expired,
  Invalid,
}

impl AuthContext {
  pub fn new(secret: impl AsRef<[u8]>, access_ttl_seconds: i64, refresh_ttl_seconds: i64) -> Self {
    Self {
      encoding_key: EncodingKey::from_secret(secret.as_ref()),
      decoding_key: DecodingKey::from_secret(secret.as_ref()),
      access_ttl_seconds,
      refresh_ttl_seconds,
    }
  }

  pub(super) fn issue_token_pair(&self, user_id: Uuid) -> Result<IssuedTokenPair, ApiError> {
    let access_token = self.issue_token(user_id, TokenType::Access, self.access_ttl_seconds)?;
    let refresh_token = self.issue_token(user_id, TokenType::Refresh, self.refresh_ttl_seconds)?;
    Ok(IssuedTokenPair {
      access_token,
      refresh_token,
      access_expires_in: self.access_ttl_seconds,
      refresh_expires_in: self.refresh_ttl_seconds,
    })
  }

  pub(super) fn issue_access_token(&self, user_id: Uuid) -> Result<IssuedAccessToken, ApiError> {
    let access_token = self.issue_token(user_id, TokenType::Access, self.access_ttl_seconds)?;
    Ok(IssuedAccessToken {
      access_token,
      access_expires_in: self.access_ttl_seconds,
    })
  }

  pub(super) fn validate_access_token(&self, token: &str) -> Result<Uuid, AccessTokenValidationError> {
    match self.decode_claims(token) {
      Ok(claims) => self
        .validate_claims(&claims, TokenType::Access)
        .map_err(|_| AccessTokenValidationError::Invalid),
      Err(error) => match error.kind() {
        ErrorKind::ExpiredSignature => Err(AccessTokenValidationError::Expired),
        _ => Err(AccessTokenValidationError::Invalid),
      },
    }
  }

  pub(super) fn validate_refresh_token(&self, token: &str) -> Option<Uuid> {
    self
      .decode_claims(token)
      .ok()
      .and_then(|claims| self.validate_claims(&claims, TokenType::Refresh).ok())
  }

  fn issue_token(&self, user_id: Uuid, token_type: TokenType, ttl_seconds: i64) -> Result<String, ApiError> {
    let now = Utc::now().timestamp();
    let claims = JwtClaims {
      sub: user_id.to_string(),
      iat: now,
      exp: now + ttl_seconds,
      token_type: token_type.as_str().to_owned(),
    };

    encode(&Header::default(), &claims, &self.encoding_key).map_err(ApiError::TokenEncoding)
  }

  fn decode_claims(&self, token: &str) -> Result<JwtClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    decode::<JwtClaims>(token, &self.decoding_key, &validation).map(|data| data.claims)
  }

  fn validate_claims(&self, claims: &JwtClaims, expected_type: TokenType) -> Result<Uuid, ()> {
    if claims.token_type != expected_type.as_str() {
      return Err(());
    }
    Uuid::parse_str(&claims.sub).map_err(|_| ())
  }
}

impl TokenType {
  fn as_str(self) -> &'static str {
    match self {
      TokenType::Access => "access",
      TokenType::Refresh => "refresh",
    }
  }
}

pub(super) async fn jwt_auth_middleware(State(auth): State<AuthContext>, mut request: Request, next: Next) -> Response {
  if request.uri().path().starts_with("/auth/") {
    return next.run(request).await;
  }

  let Some(token) = bearer_token_from_headers(request.headers()) else {
    return unauthorized_response(models::ErrorCode::TokenInvalid, "Access token is missing or malformed");
  };

  let user_id = match auth.validate_access_token(token) {
    Ok(user_id) => user_id,
    Err(AccessTokenValidationError::Expired) => {
      return unauthorized_response(models::ErrorCode::TokenExpired, "Access token has expired");
    }
    Err(AccessTokenValidationError::Invalid) => {
      return unauthorized_response(models::ErrorCode::TokenInvalid, "Access token is invalid");
    }
  };

  request
    .extensions_mut()
    .insert(AuthenticatedUserId(user_id.to_string()));

  let mut response = next.run(request).await;
  response
    .extensions_mut()
    .insert(AuthenticatedUserId(user_id.to_string()));
  response
}

fn bearer_token_from_headers(headers: &HeaderMap) -> Option<&str> {
  let header_value = headers.get(AUTHORIZATION)?.to_str().ok()?;
  header_value.strip_prefix(BEARER_PREFIX)
}

fn unauthorized_response(error_code: models::ErrorCode, message: &str) -> Response {
  let payload = error_response(error_code, message);
  let body = match serde_json::to_vec(&payload) {
    Ok(value) => value,
    Err(_) => b"{\"error_code\":\"TOKEN_INVALID\",\"message\":\"Unauthorized\"}".to_vec(),
  };

  let mut response = Response::new(Body::from(body));
  *response.status_mut() = StatusCode::UNAUTHORIZED;
  response
    .headers_mut()
    .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
  response
}
