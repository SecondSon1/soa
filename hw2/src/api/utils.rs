use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

use super::ApiError;
use crate::models;
use crate::types::Nullable;

pub(super) fn is_bad_input_db_error(error: &sqlx::Error) -> bool {
  let Some(db_error) = error.as_database_error() else {
    return false;
  };

  matches!(
    db_error.code().as_deref(),
    Some("23502" | "23503" | "23505" | "23514" | "22P02" | "22003")
  )
}

pub(super) fn nullable_to_option(nullable: Option<&Nullable<String>>) -> Option<String> {
  match nullable {
    Some(Nullable::Present(value)) => Some(value.clone()),
    Some(Nullable::Null) | None => None,
  }
}

pub(super) fn decimal_from_f64(value: f64) -> Result<Decimal, ApiError> {
  Decimal::from_f64(value).ok_or(ApiError::InvalidPrice(value))
}

pub(super) fn f64_from_decimal(value: Decimal) -> Result<f64, ApiError> {
  value.to_f64().ok_or(ApiError::InvalidPriceFromDb(value))
}

pub(super) fn status_to_db(status: models::ProductStatus) -> &'static str {
  match status {
    models::ProductStatus::Active => "ACTIVE",
    models::ProductStatus::Inactive => "INACTIVE",
    models::ProductStatus::Archived => "ARCHIVED",
  }
}

pub(super) fn status_from_db(status: &str) -> Result<models::ProductStatus, ApiError> {
  match status {
    "ACTIVE" => Ok(models::ProductStatus::Active),
    "INACTIVE" => Ok(models::ProductStatus::Inactive),
    "ARCHIVED" => Ok(models::ProductStatus::Archived),
    other => Err(ApiError::InvalidStatus(other.to_owned())),
  }
}
