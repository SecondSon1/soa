use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::metrics::MetricResult;

pub async fn run_migration(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS metrics (
            date DATE NOT NULL,
            metric_name TEXT NOT NULL,
            dimension TEXT NOT NULL DEFAULT '',
            value DOUBLE PRECISION NOT NULL,
            computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (date, metric_name, dimension)
        )",
    )
    .execute(pool)
    .await?;
    info!("PostgreSQL migration complete");
    Ok(())
}

pub async fn upsert_metrics(pool: &PgPool, results: &[MetricResult]) -> Result<()> {
    for attempt in 1..=3 {
        match do_upsert(pool, results).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt == 3 {
                    return Err(e);
                }
                warn!(attempt, "PostgreSQL upsert failed, retrying: {:#}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
    unreachable!()
}

async fn do_upsert(pool: &PgPool, results: &[MetricResult]) -> Result<()> {
    let mut tx = pool.begin().await?;
    for r in results {
        sqlx::query(
            "INSERT INTO metrics (date, metric_name, dimension, value, computed_at)
             VALUES ($1, $2, $3, $4, NOW())
             ON CONFLICT (date, metric_name, dimension)
             DO UPDATE SET value = EXCLUDED.value, computed_at = EXCLUDED.computed_at",
        )
        .bind(r.date)
        .bind(&r.metric_name)
        .bind(&r.dimension)
        .bind(r.value)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
