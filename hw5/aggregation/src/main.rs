mod metrics;
mod postgres;
mod routes;
mod s3_export;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

pub struct AppState {
    pub ch: clickhouse::Client,
    pub pg: sqlx::PgPool,
    pub s3: Option<s3_export::S3Config>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let ch_url = std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://localhost:8123".into());
    let pg_url = std::env::var("POSTGRES_URL")
        .unwrap_or_else(|_| "postgres://cinema:cinema@localhost:5432/cinema".into());
    let port: u16 = std::env::var("HTTP_PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()
        .context("invalid HTTP_PORT")?;
    let interval_secs: u64 = std::env::var("AGGREGATION_INTERVAL_SECS")
        .unwrap_or_else(|_| "60".into())
        .parse()
        .context("invalid AGGREGATION_INTERVAL_SECS")?;

    let s3_config = match (
        std::env::var("S3_ENDPOINT"),
        std::env::var("S3_BUCKET"),
        std::env::var("S3_ACCESS_KEY"),
        std::env::var("S3_SECRET_KEY"),
    ) {
        (Ok(endpoint), Ok(bucket), Ok(access_key), Ok(secret_key)) => {
            info!("S3 export enabled: {} / {}", endpoint, bucket);
            Some(s3_export::S3Config {
                endpoint,
                bucket,
                access_key,
                secret_key,
            })
        }
        _ => {
            info!("S3 export disabled (env vars not set)");
            None
        }
    };

    let ch = clickhouse::Client::default().with_url(&ch_url);
    info!("connected to ClickHouse at {}", ch_url);

    let pg = sqlx::PgPool::connect(&pg_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    info!("connected to PostgreSQL");

    postgres::run_migration(&pg).await?;

    let state = Arc::new(AppState {
        ch,
        pg,
        s3: s3_config,
    });

    let scheduler_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            if let Err(e) = run_aggregation_all(&scheduler_state).await {
                tracing::error!("scheduled aggregation failed: {:#}", e);
            }
        }
    });

    let app = routes::router(state);
    let addr = format!("0.0.0.0:{}", port);
    info!("starting aggregation HTTP server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn run_aggregation_for_date(
    state: &AppState,
    date: NaiveDate,
) -> Result<Vec<metrics::MetricResult>> {
    let start = Instant::now();
    info!(%date, "aggregation cycle start");

    let results = metrics::compute_for_date(&state.ch, date).await?;
    metrics::write_to_clickhouse(&state.ch, &results).await?;
    postgres::upsert_metrics(&state.pg, &results).await?;

    let elapsed = start.elapsed();
    info!(
        %date,
        records = results.len(),
        elapsed_ms = elapsed.as_millis() as u64,
        "aggregation cycle complete"
    );
    Ok(results)
}

pub async fn export_for_date(state: &AppState, date: NaiveDate) -> Result<()> {
    if let Some(ref s3) = state.s3 {
        s3_export::export_date(&state.pg, s3, date).await?;
    }
    Ok(())
}

async fn run_aggregation_all(state: &AppState) -> Result<()> {
    let start = Instant::now();
    info!("scheduled aggregation start");

    let dates = metrics::get_all_dates(&state.ch).await?;
    let mut total_records = 0usize;

    for date in &dates {
        let results = run_aggregation_for_date(state, *date).await?;
        total_records += results.len();
    }

    if state.s3.is_some() {
        for date in &dates {
            if let Err(e) = export_for_date(state, *date).await {
                tracing::error!(%date, "S3 export failed: {:#}", e);
            }
        }
    }

    let elapsed = start.elapsed();
    info!(
        dates = dates.len(),
        total_records,
        elapsed_ms = elapsed.as_millis() as u64,
        "scheduled aggregation complete"
    );
    Ok(())
}
