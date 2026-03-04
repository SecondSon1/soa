#![allow(
  missing_docs,
  trivial_casts,
  unused_variables,
  unused_mut,
  unused_extern_crates,
  non_camel_case_types,
  unused_imports,
  unused_attributes
)]
#![allow(
  clippy::derive_partial_eq_without_eq,
  clippy::disallowed_names,
  clippy::too_many_arguments
)]

pub const BASE_PATH: &str = "";
pub const API_VERSION: &str = "1.0.0";

#[path = "openapi_generated/src/apis/mod.rs"]
pub mod apis;
#[path = "openapi_generated/src/header.rs"]
pub(crate) mod header;
#[path = "openapi_generated/src/models.rs"]
pub mod models;
#[path = "openapi_generated/src/server/mod.rs"]
pub mod server;
#[path = "openapi_generated/src/types.rs"]
pub mod types;

pub mod api;
