use anyhow::{Context, Result};
use apache_avro::Schema;

pub const AVRO_SCHEMA_STR: &str = include_str!("../../avro/movie_event.avsc");
const SUBJECT: &str = "movie-events-value";

pub fn parse_schema() -> Result<Schema> {
    Schema::parse_str(AVRO_SCHEMA_STR).context("failed to parse Avro schema")
}

pub async fn register_schema(registry_url: &str) -> Result<u32> {
    let client = reqwest::Client::new();
    let url = format!("{}/subjects/{}/versions", registry_url, SUBJECT);
    let body = serde_json::json!({
        "schemaType": "AVRO",
        "schema": AVRO_SCHEMA_STR
    });
    let resp = client
        .post(&url)
        .header("Content-Type", "application/vnd.schemaregistry.v1+json")
        .json(&body)
        .send()
        .await
        .context("failed to reach Schema Registry")?;

    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("Schema Registry returned {}: {}", status, text);
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).context("failed to parse Schema Registry response")?;
    let id = parsed["id"]
        .as_u64()
        .context("Schema Registry response missing 'id' field")?;
    Ok(id as u32)
}

pub fn encode_confluent(schema_id: u32, avro_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + avro_bytes.len());
    buf.push(0u8);
    buf.extend_from_slice(&schema_id.to_be_bytes());
    buf.extend_from_slice(avro_bytes);
    buf
}
