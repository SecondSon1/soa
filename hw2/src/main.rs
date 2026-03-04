use std::net::SocketAddr;
use std::{env, time::Duration};

use anyhow::Context;
use marketplace::api::{ApiService, AuthContext, with_api_middlewares};
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
  let jwt_secret = env::var("JWT_SECRET").expect("No JWT_SECRET provided");
  let access_ttl_minutes = parse_env_i64("JWT_ACCESS_TTL_MINUTES", 15)?;
  let refresh_ttl_days = parse_env_i64("JWT_REFRESH_TTL_DAYS", 7)?;
  if !(15..=30).contains(&access_ttl_minutes) {
    anyhow::bail!("JWT_ACCESS_TTL_MINUTES must be in range 15..=30");
  }
  if !(7..=30).contains(&refresh_ttl_days) {
    anyhow::bail!("JWT_REFRESH_TTL_DAYS must be in range 7..=30");
  }

  let pool = PgPoolOptions::new()
    .max_connections(4)
    .acquire_timeout(Duration::from_secs(5))
    .connect(&database_url)
    .await
    .with_context(|| format!("failed to connect to postgres: {database_url}"))?;

  let auth_context = AuthContext::new(jwt_secret, access_ttl_minutes * 60, refresh_ttl_days * 24 * 60 * 60);

  let service = ApiService::new(pool, auth_context.clone());
  let app = with_api_middlewares(server::new(service), auth_context);

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

fn parse_env_i64(name: &str, default: i64) -> anyhow::Result<i64> {
  match env::var(name) {
    Ok(raw) => {
      let parsed = raw
        .parse::<i64>()
        .with_context(|| format!("invalid integer value in {name}: {raw}"))?;
      if parsed <= 0 {
        anyhow::bail!("{name} must be positive");
      }
      Ok(parsed)
    }
    Err(env::VarError::NotPresent) => Ok(default),
    Err(err) => Err(anyhow::anyhow!("failed to read {name}: {err}")),
  }
}
