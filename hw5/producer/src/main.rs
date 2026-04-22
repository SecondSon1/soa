mod generator;
mod routes;

use anyhow::{Context, Result};
use common::apache_avro::Schema;
use common::kafka::create_producer;
use common::rdkafka::producer::FutureProducer;
use common::schema;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub producer: FutureProducer,
    pub schema: Schema,
    pub schema_id: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let brokers: String = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let registry_url: String =
        std::env::var("SCHEMA_REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8081".into());
    let port: u16 = std::env::var("HTTP_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .context("invalid HTTP_PORT")?;

    let avro_schema = schema::parse_schema()?;
    info!("parsed Avro schema");

    let schema_id = wait_and_register_schema(&registry_url).await?;
    info!(schema_id, "registered schema with Schema Registry");

    let producer = create_producer(&brokers)?;
    info!("created Kafka producer");

    let state = AppState {
        producer,
        schema: avro_schema,
        schema_id,
    };

    let app = routes::router(Arc::new(state));
    let addr = format!("0.0.0.0:{}", port);
    info!("starting HTTP server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn wait_and_register_schema(registry_url: &str) -> Result<u32> {
    for attempt in 1..=30 {
        match schema::register_schema(registry_url).await {
            Ok(id) => return Ok(id),
            Err(e) => {
                if attempt == 30 {
                    return Err(e).context("Schema Registry not available after 30 attempts");
                }
                tracing::warn!(attempt, "Schema Registry not ready: {:#}", e);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    unreachable!()
}
