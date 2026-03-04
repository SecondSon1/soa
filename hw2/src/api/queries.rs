use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgArguments;
use sqlx::query::{QueryAs, QueryScalar};
use sqlx::{FromRow, Postgres};
use uuid::Uuid;

use crate::api::model::{ProductRow, UserRow};

pub(super) struct Queries;

impl Queries {
  pub fn insert_user(email: String, password_hash: String) -> QueryAs<'static, Postgres, UserRow, PgArguments> {
    sqlx::query_as::<_, UserRow>(
      "INSERT INTO users (email, password_hash) \
       VALUES ($1, $2) \
       RETURNING id, email, password_hash, created_at",
    )
    .bind(email)
    .bind(password_hash)
  }

  pub fn select_user_by_email(email: String) -> QueryAs<'static, Postgres, UserRow, PgArguments> {
    sqlx::query_as::<_, UserRow>(
      "SELECT id, email, password_hash, created_at \
       FROM users \
       WHERE email = $1",
    )
    .bind(email)
  }

  pub fn insert_product(
    name: String,
    description: Option<String>,
    price: Decimal,
    stock: i32,
    category: String,
    status: String,
  ) -> QueryAs<'static, Postgres, ProductRow, PgArguments> {
    sqlx::query_as::<_, ProductRow>(
      "INSERT INTO products (name, description, price, stock, category, status) \
       VALUES ($1, $2, $3, $4, $5, $6::product_status) \
       RETURNING id, name, description, price, stock, category, status::text AS status, created_at, updated_at",
    )
    .bind(name)
    .bind(description)
    .bind(price)
    .bind(stock)
    .bind(category)
    .bind(status)
  }

  pub fn soft_delete_product(id: Uuid) -> QueryScalar<'static, Postgres, Uuid, PgArguments> {
    sqlx::query_scalar::<_, Uuid>(
      "UPDATE products SET status = 'ARCHIVED'::product_status, updated_at = NOW() WHERE id = $1 RETURNING id",
    )
    .bind(id)
  }

  pub fn select_product_by_id(id: Uuid) -> QueryAs<'static, Postgres, ProductRow, PgArguments> {
    sqlx::query_as::<_, ProductRow>(
      "SELECT id, name, description, price, stock, category, status::text AS status, created_at, updated_at \
       FROM products WHERE id = $1",
    )
    .bind(id)
  }

  pub fn select_products(
    status_filter: Option<String>,
    category_filter: Option<String>,
    size: i64,
    offset: i64,
  ) -> QueryAs<'static, Postgres, ProductRow, PgArguments> {
    sqlx::query_as::<_, ProductRow>(
      "SELECT id, name, description, price, stock, category, status::text AS status, created_at, updated_at \
       FROM products \
       WHERE ($1::product_status IS NULL OR status = $1::product_status) \
         AND ($2::text IS NULL OR category = $2) \
       ORDER BY created_at DESC \
       LIMIT $3 OFFSET $4",
    )
    .bind(status_filter)
    .bind(category_filter)
    .bind(size)
    .bind(offset)
  }

  pub fn count_products(
    status_filter: Option<String>,
    category_filter: Option<String>,
  ) -> QueryScalar<'static, Postgres, i64, PgArguments> {
    sqlx::query_scalar::<_, i64>(
      "SELECT COUNT(*)::bigint \
       FROM products \
       WHERE ($1::product_status IS NULL OR status = $1::product_status) \
         AND ($2::text IS NULL OR category = $2)",
    )
    .bind(status_filter)
    .bind(category_filter)
  }

  pub fn update_product(
    id: Uuid,
    name: String,
    description: Option<String>,
    price: Decimal,
    stock: i32,
    category: String,
    status: String,
  ) -> QueryAs<'static, Postgres, ProductRow, PgArguments> {
    sqlx::query_as::<_, ProductRow>(
      "UPDATE products \
       SET name = $2, description = $3, price = $4, stock = $5, category = $6, status = $7::product_status, updated_at = NOW() \
       WHERE id = $1 \
       RETURNING id, name, description, price, stock, category, status::text AS status, created_at, updated_at",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(price)
    .bind(stock)
    .bind(category)
    .bind(status)
  }
}
