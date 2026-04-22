use anyhow::{Context, Result};
use apache_avro::Schema;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use tracing::info;

use crate::event::MovieEvent;
use crate::schema::encode_confluent;

pub const TOPIC: &str = "movie-events";

pub fn create_producer(brokers: &str) -> Result<FutureProducer> {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("acks", "all")
        .set("retries", "5")
        .set("retry.backoff.ms", "100")
        .set("retry.backoff.max.ms", "2000")
        .set("message.timeout.ms", "10000")
        .create()
        .context("failed to create Kafka producer")
}

pub async fn publish_event(
    producer: &FutureProducer,
    schema: &Schema,
    schema_id: u32,
    event: &MovieEvent,
) -> Result<()> {
    let record = event.to_avro_record(schema)?;
    let value = apache_avro::types::Value::from(record);
    let raw_bytes = apache_avro::to_avro_datum(schema, value)?;
    let payload = encode_confluent(schema_id, &raw_bytes);

    let key = event.user_id.to_string();
    let key = key.as_bytes();
    let kafka_record = FutureRecord::to(TOPIC).key(key).payload(&payload);

    producer
        .send(kafka_record, Duration::from_secs(5))
        .await
        .map_err(|(err, _)| anyhow::anyhow!("Kafka send failed: {}", err))?;

    info!(
        event_id = %event.event_id,
        event_type = ?event.event_type,
        timestamp = event.timestamp,
        "published event"
    );
    Ok(())
}
