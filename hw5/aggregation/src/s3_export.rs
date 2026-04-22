use anyhow::Result;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct MetricRow {
    date: NaiveDate,
    metric_name: String,
    dimension: String,
    value: f64,
    computed_at: chrono::DateTime<chrono::Utc>,
}

pub async fn export_date(pool: &PgPool, config: &S3Config, date: NaiveDate) -> Result<()> {
    let rows = sqlx::query_as::<_, MetricRow>(
        "SELECT date, metric_name, dimension, value, computed_at \
         FROM metrics WHERE date = $1 ORDER BY metric_name, dimension",
    )
    .bind(date)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        tracing::warn!(%date, "no metrics to export");
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&rows)?;
    let path = format!("daily/{}/aggregates.json", date.format("%Y-%m-%d"));

    upload_to_s3(config, &path, json.as_bytes()).await?;
    tracing::info!(%date, rows = rows.len(), "exported metrics to S3");
    Ok(())
}

async fn upload_to_s3(config: &S3Config, path: &str, data: &[u8]) -> Result<()> {
    use s3::creds::Credentials;
    use s3::{Bucket, Region};

    let region = Region::Custom {
        region: "us-east-1".to_owned(),
        endpoint: config.endpoint.clone(),
    };
    let credentials = Credentials::new(
        Some(&config.access_key),
        Some(&config.secret_key),
        None,
        None,
        None,
    )?;
    let bucket = Bucket::new(&config.bucket, region, credentials)?.with_path_style();
    bucket.put_object(path, data).await?;
    Ok(())
}
