use std::net::SocketAddr;
use std::{env, time::Duration};

use anyhow::Context;
use marketplace::api::{ApiService, with_api_logging};
use marketplace::server;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with(tracing_subscriber::fmt::layer().json().flatten_event(true))
    .init();

  let database_url = env::var("DATABASE_URL").expect("No DATABASE_URL provided");

  let pool = PgPoolOptions::new()
    .max_connections(4)
    .acquire_timeout(Duration::from_secs(5))
    .connect(&database_url)
    .await
    .with_context(|| format!("failed to connect to postgres: {database_url}"))?;

  let service = ApiService::new(pool);
  let app = with_api_logging(server::new(service));

  let bind_addr: SocketAddr = env::var("APP_ADDR")
    .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
    .parse()
    .context("invalid APP_ADDR")?;

  let listener = tokio::net::TcpListener::bind(bind_addr)
    .await
    .context("failed to bind TCP listener")?;

  tracing::info!(log_type = "app_event", %bind_addr, "starting server");
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server failure")?;

  Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
  let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    .expect("failed to install SIGTERM handler");

  tokio::select! {
    result = tokio::signal::ctrl_c() => {
      result.expect("failed to install Ctrl+C handler");
    }
    _ = terminate.recv() => {}
  }

  tracing::info!(log_type = "app_event", "shutdown signal received, stopping server");
}

#[cfg(windows)]
async fn shutdown_signal() {
  panic!("Windows is not supported");
}

#[cfg(not(any(unix, windows)))]
async fn shutdown_signal() {
  tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
  tracing::info!(log_type = "app_event", "shutdown signal received, stopping server");
}
