use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::FromRow;
use uuid::Uuid;

use super::ApiError;
use super::utils::{f64_from_decimal, status_from_db};
use crate::models;
use crate::types::Nullable;

#[derive(Debug, FromRow)]
pub(super) struct ProductRow {
  pub id: Uuid,
  pub name: String,
  pub description: Option<String>,
  pub price: Decimal,
  pub stock: i32,
  pub category: String,
  pub status: String,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl ProductRow {
  pub(super) fn to_model(self) -> Result<models::ProductResponse, ApiError> {
    Ok(models::ProductResponse {
      id: self.id,
      name: self.name,
      description: Some(match self.description {
        Some(value) => Nullable::Present(value),
        None => Nullable::Null,
      }),
      price: f64_from_decimal(self.price)?,
      stock: self.stock,
      category: self.category,
      status: status_from_db(&self.status)?,
      created_at: self.created_at,
      updated_at: self.updated_at,
    })
  }
}

#[derive(Debug, FromRow)]
pub(super) struct UserRow {
  pub id: Uuid,
  pub email: String,
  pub password_hash: String,
  pub created_at: DateTime<Utc>,
}
