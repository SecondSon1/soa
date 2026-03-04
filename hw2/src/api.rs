mod auth;
mod logging;
mod model;
mod queries;
mod utils;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use axum::{body::Body, response::Response};
use axum_extra::extract::CookieJar;
use headers::Host;
use http::HeaderMap;
use http::{Method, StatusCode};
use jsonwebtoken::errors::Error as JwtError;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

pub use self::auth::AuthContext;
use self::model::{ProductRow, UserRow};
use self::queries::Queries;
use self::utils::{
  decimal_from_f64, error_response, is_bad_input_db_error, nullable_to_option, status_to_db, validation_error_response,
};
use crate::apis::auth::{Auth, LoginUserResponse, RefreshTokenResponse, RegisterUserResponse};
use crate::apis::products::{
  CreateProductResponse, DeleteProductResponse, GetProductByIdResponse, ListProductsResponse, Products,
  UpdateProductResponse,
};
use crate::{apis, models};

#[derive(Clone)]
pub struct ApiService {
  pool: PgPool,
  auth: AuthContext,
}

impl ApiService {
  pub fn new(pool: PgPool, auth: AuthContext) -> Self {
    Self { pool, auth }
  }

  pub fn auth_context(&self) -> AuthContext {
    self.auth.clone()
  }
}

pub fn with_api_middlewares(router: axum::Router, auth_context: AuthContext) -> axum::Router {
  router
    .layer(axum::middleware::from_fn_with_state(
      auth_context,
      auth::jwt_auth_middleware,
    ))
    .layer(axum::middleware::from_fn(logging::api_logging_middleware))
}

impl AsRef<ApiService> for ApiService {
  fn as_ref(&self) -> &ApiService {
    self
  }
}

#[derive(Debug)]
pub enum ApiError {
  Db(sqlx::Error),
  InvalidStatus(String),
  InvalidPrice(f64),
  InvalidPriceFromDb(Decimal),
  PasswordHash(argon2::password_hash::Error),
  TokenEncoding(JwtError),
}

impl From<sqlx::Error> for ApiError {
  fn from(value: sqlx::Error) -> Self {
    Self::Db(value)
  }
}

impl From<argon2::password_hash::Error> for ApiError {
  fn from(value: argon2::password_hash::Error) -> Self {
    Self::PasswordHash(value)
  }
}

impl From<JwtError> for ApiError {
  fn from(value: JwtError) -> Self {
    Self::TokenEncoding(value)
  }
}

#[async_trait]
impl apis::ApiAuthBasic for ApiService {
  type Claims = ();

  async fn extract_claims_from_auth_header(
    &self,
    _kind: apis::BasicAuthKind,
    _headers: &HeaderMap,
    _key: &str,
  ) -> Option<Self::Claims> {
    Some(())
  }
}

#[async_trait]
impl apis::ErrorHandler<ApiError> for ApiService {
  async fn handle_error(
    &self,
    method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    error: ApiError,
  ) -> Result<Response, StatusCode> {
    error!(log_type = "app_event", ?method, ?error, "unhandled API error");
    Response::builder()
      .status(StatusCode::INTERNAL_SERVER_ERROR)
      .body(Body::empty())
      .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
  }
}

#[async_trait]
impl Products<ApiError> for ApiService {
  type Claims = ();

  async fn create_product(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    _claims: &Self::Claims,
    body: &models::ProductCreate,
  ) -> Result<CreateProductResponse, ApiError> {
    match self.create_product_in_db(body).await {
      Ok(product) => Ok(CreateProductResponse::Status201_ProductCreated(product)),
      Err(ApiError::Db(db_error)) if is_bad_input_db_error(&db_error) => Ok(
        CreateProductResponse::Status400_RequestValidationFailed(validation_error_response("request", "invalid input")),
      ),
      Err(other) => Err(other),
    }
  }

  async fn delete_product(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    _claims: &Self::Claims,
    path_params: &models::DeleteProductPathParams,
  ) -> Result<DeleteProductResponse, ApiError> {
    let updated = Queries::soft_delete_product(path_params.id)
      .fetch_optional(&self.pool)
      .await?;

    if updated.is_some() {
      Ok(DeleteProductResponse::Status204_ProductDeleted)
    } else {
      Ok(DeleteProductResponse::Status404_ProductNotFound(error_response(
        models::ErrorCode::ProductNotFound,
        "Product not found",
      )))
    }
  }

  async fn get_product_by_id(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    _claims: &Self::Claims,
    path_params: &models::GetProductByIdPathParams,
  ) -> Result<GetProductByIdResponse, ApiError> {
    let row = Queries::select_product_by_id(path_params.id)
      .fetch_optional(&self.pool)
      .await?;

    if let Some(row) = row {
      Ok(GetProductByIdResponse::Status200_ProductFound(row.to_model()?))
    } else {
      Ok(GetProductByIdResponse::Status404_ProductNotFound(error_response(
        models::ErrorCode::ProductNotFound,
        "Product not found",
      )))
    }
  }

  async fn list_products(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    _claims: &Self::Claims,
    query_params: &models::ListProductsQueryParams,
  ) -> Result<ListProductsResponse, ApiError> {
    let page = query_params.page.unwrap_or(0).max(0) as i64;
    let size = query_params.size.unwrap_or(20).max(1) as i64;
    let offset = page * size;

    let status_filter = query_params.status.map(|status| status_to_db(status).to_owned());
    let category_filter = query_params.category.clone();

    let rows = Queries::select_products(status_filter.clone(), category_filter.clone(), size, offset)
      .fetch_all(&self.pool)
      .await?;

    let total = Queries::count_products(status_filter, category_filter)
      .fetch_one(&self.pool)
      .await?;

    let items = rows
      .into_iter()
      .map(ProductRow::to_model)
      .collect::<Result<Vec<_>, _>>()?;

    Ok(ListProductsResponse::Status200_PaginatedProductList(
      models::ProductPageResponse {
        items,
        total_elements: total,
        page: page as u32,
        size: size as u32,
      },
    ))
  }

  async fn update_product(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    _claims: &Self::Claims,
    path_params: &models::UpdateProductPathParams,
    body: &models::ProductUpdate,
  ) -> Result<UpdateProductResponse, ApiError> {
    match self.update_product_in_db(path_params.id, body).await {
      Ok(Some(product)) => Ok(UpdateProductResponse::Status200_ProductUpdated(product)),
      Ok(None) => Ok(UpdateProductResponse::Status404_ProductNotFound(error_response(
        models::ErrorCode::ProductNotFound,
        "Product not found",
      ))),
      Err(ApiError::Db(db_error)) if is_bad_input_db_error(&db_error) => Ok(
        UpdateProductResponse::Status400_RequestValidationFailed(validation_error_response("request", "invalid input")),
      ),
      Err(other) => Err(other),
    }
  }
}

#[async_trait]
impl Auth<ApiError> for ApiService {
  async fn login_user(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    body: &models::AuthLoginRequest,
  ) -> Result<LoginUserResponse, ApiError> {
    let email = normalize_email(&body.email);
    let user = Queries::select_user_by_email(email).fetch_optional(&self.pool).await?;

    let Some(user) = user else {
      return Ok(LoginUserResponse::Status401_AccessTokenInvalid(error_response(
        models::ErrorCode::TokenInvalid,
        "Invalid email or password",
      )));
    };

    if !verify_password(&user.password_hash, &body.password)? {
      return Ok(LoginUserResponse::Status401_AccessTokenInvalid(error_response(
        models::ErrorCode::TokenInvalid,
        "Invalid email or password",
      )));
    }

    let token_pair = self.auth.issue_token_pair(user.id)?;
    Ok(LoginUserResponse::Status200_AccessAndRefreshTokensIssued(
      models::AuthTokenPairResponse {
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        token_type: "Bearer".to_owned(),
        access_expires_in: token_pair.access_expires_in,
        refresh_expires_in: token_pair.refresh_expires_in,
      },
    ))
  }

  async fn refresh_token(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    body: &models::AuthRefreshRequest,
  ) -> Result<RefreshTokenResponse, ApiError> {
    let Some(user_id) = self.auth.validate_refresh_token(&body.refresh_token) else {
      return Ok(RefreshTokenResponse::Status401_RefreshTokenInvalid(error_response(
        models::ErrorCode::RefreshTokenInvalid,
        "Refresh token is invalid",
      )));
    };

    let access_token = self.auth.issue_access_token(user_id)?;
    Ok(RefreshTokenResponse::Status200_NewAccessTokenIssued(
      models::AuthAccessTokenResponse {
        access_token: access_token.access_token,
        token_type: "Bearer".to_owned(),
        access_expires_in: access_token.access_expires_in,
      },
    ))
  }

  async fn register_user(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    body: &models::AuthRegisterRequest,
  ) -> Result<RegisterUserResponse, ApiError> {
    let email = normalize_email(&body.email);
    let password_hash = hash_password(&body.password)?;

    match Queries::insert_user(email, password_hash).fetch_one(&self.pool).await {
      Ok(user) => Ok(RegisterUserResponse::Status201_UserRegistered(to_register_response(
        user,
      ))),
      Err(db_error) if is_bad_input_db_error(&db_error) => Ok(RegisterUserResponse::Status400_RequestValidationFailed(
        validation_error_response("email", "invalid or already exists"),
      )),
      Err(db_error) => Err(ApiError::Db(db_error)),
    }
  }
}

impl ApiService {
  async fn create_product_in_db(&self, body: &models::ProductCreate) -> Result<models::ProductResponse, ApiError> {
    let price = decimal_from_f64(body.price)?;
    let description = nullable_to_option(body.description.as_ref());

    let row = Queries::insert_product(
      body.name.clone(),
      description,
      price,
      body.stock as i32,
      body.category.clone(),
      status_to_db(body.status).to_owned(),
    )
    .fetch_one(&self.pool)
    .await?;

    row.to_model()
  }

  async fn update_product_in_db(
    &self,
    id: Uuid,
    body: &models::ProductUpdate,
  ) -> Result<Option<models::ProductResponse>, ApiError> {
    let price = decimal_from_f64(body.price)?;
    let description = nullable_to_option(body.description.as_ref());

    let row = Queries::update_product(
      id,
      body.name.clone(),
      description,
      price,
      body.stock as i32,
      body.category.clone(),
      status_to_db(body.status).to_owned(),
    )
    .fetch_optional(&self.pool)
    .await?;

    row.map(ProductRow::to_model).transpose()
  }
}

fn normalize_email(email: &str) -> String {
  email.trim().to_ascii_lowercase()
}

fn hash_password(password: &str) -> Result<String, ApiError> {
  let salt = SaltString::generate(&mut OsRng);
  let hash = Argon2::default().hash_password(password.as_bytes(), &salt)?;
  Ok(hash.to_string())
}

fn verify_password(password_hash: &str, password: &str) -> Result<bool, ApiError> {
  let parsed_hash = PasswordHash::new(password_hash)?;
  Ok(
    Argon2::default()
      .verify_password(password.as_bytes(), &parsed_hash)
      .is_ok(),
  )
}

fn to_register_response(row: UserRow) -> models::AuthRegisterResponse {
  models::AuthRegisterResponse {
    user_id: row.id,
    email: row.email,
    created_at: row.created_at,
  }
}
