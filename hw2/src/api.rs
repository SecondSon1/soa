mod logging;
mod model;
mod queries;
mod utils;

use async_trait::async_trait;
use axum::{body::Body, response::Response};
use axum_extra::extract::CookieJar;
use headers::Host;
use http::{Method, StatusCode};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

use self::model::ProductRow;
use self::queries::Queries;
use self::utils::{decimal_from_f64, is_bad_input_db_error, nullable_to_option, status_to_db};
use crate::apis::products::{
  CreateProductResponse, DeleteProductResponse, GetProductByIdResponse, ListProductsResponse, Products,
  UpdateProductResponse,
};
use crate::{apis, models};

#[derive(Debug, Clone)]
pub struct ApiService {
  pool: PgPool,
}

impl ApiService {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

pub fn with_api_logging(router: axum::Router) -> axum::Router {
  router.layer(axum::middleware::from_fn(logging::api_logging_middleware))
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
}

impl From<sqlx::Error> for ApiError {
  fn from(value: sqlx::Error) -> Self {
    Self::Db(value)
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
  async fn create_product(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    body: &models::ProductCreate,
  ) -> Result<CreateProductResponse, ApiError> {
    match self.create_product_in_db(body).await {
      Ok(product) => Ok(CreateProductResponse::Status201_ProductCreated(product)),
      Err(ApiError::Db(db_error)) if is_bad_input_db_error(&db_error) => {
        Ok(CreateProductResponse::Status400_InvalidInput)
      }
      Err(other) => Err(other),
    }
  }

  async fn delete_product(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    path_params: &models::DeleteProductPathParams,
  ) -> Result<DeleteProductResponse, ApiError> {
    let updated = Queries::soft_delete_product(path_params.id)
      .fetch_optional(&self.pool)
      .await?;

    if updated.is_some() {
      Ok(DeleteProductResponse::Status204_ProductDeleted)
    } else {
      Ok(DeleteProductResponse::Status404_ProductNotFound)
    }
  }

  async fn get_product_by_id(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
    path_params: &models::GetProductByIdPathParams,
  ) -> Result<GetProductByIdResponse, ApiError> {
    let row = Queries::select_product_by_id(path_params.id)
      .fetch_optional(&self.pool)
      .await?;

    if let Some(row) = row {
      Ok(GetProductByIdResponse::Status200_ProductFound(row.to_model()?))
    } else {
      Ok(GetProductByIdResponse::Status404_ProductNotFound)
    }
  }

  async fn list_products(
    &self,
    _method: &Method,
    _host: &Host,
    _cookies: &CookieJar,
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
    path_params: &models::UpdateProductPathParams,
    body: &models::ProductUpdate,
  ) -> Result<UpdateProductResponse, ApiError> {
    match self.update_product_in_db(path_params.id, body).await {
      Ok(Some(product)) => Ok(UpdateProductResponse::Status200_ProductUpdated(product)),
      Ok(None) => Ok(UpdateProductResponse::Status404_ProductNotFound),
      Err(ApiError::Db(db_error)) if is_bad_input_db_error(&db_error) => {
        Ok(UpdateProductResponse::Status400_InvalidInput)
      }
      Err(other) => Err(other),
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
