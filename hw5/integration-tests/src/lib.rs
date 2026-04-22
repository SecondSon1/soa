use anyhow::{Context, Result, ensure};
use clickhouse::Row;
use common::apache_avro;
use common::event::{DeviceType, EventType, MovieEvent};
use common::kafka::TOPIC;
use common::rdkafka::config::ClientConfig;
use common::rdkafka::consumer::{BaseConsumer, Consumer};
use common::rdkafka::{Message, Offset, TopicPartitionList};
use common::schema::AVRO_SCHEMA_STR;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

const DEFAULT_PRODUCER_URL: &str = "http://127.0.0.1:3000";
const DEFAULT_CLICKHOUSE_URL: &str = "http://127.0.0.1:8123";
const DEFAULT_SCHEMA_REGISTRY_URL: &str = "http://127.0.0.1:8081";
const DEFAULT_KAFKA_BROKERS: &str = "localhost:9092,localhost:9093";
const SCHEMA_SUBJECT: &str = "movie-events-value";
const KAFKA_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub struct TestEnv {
    pub producer_url: String,
    pub clickhouse_url: String,
    pub schema_registry_url: String,
    pub kafka_brokers: String,
    http: reqwest::Client,
}

impl TestEnv {
    pub fn from_env() -> Result<Self> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .build()
            .context("failed to build HTTP client for integration tests")?;

        Ok(Self {
            producer_url: std::env::var("PRODUCER_URL")
                .unwrap_or_else(|_| DEFAULT_PRODUCER_URL.into()),
            clickhouse_url: std::env::var("CLICKHOUSE_URL")
                .unwrap_or_else(|_| DEFAULT_CLICKHOUSE_URL.into()),
            schema_registry_url: std::env::var("SCHEMA_REGISTRY_URL")
                .unwrap_or_else(|_| DEFAULT_SCHEMA_REGISTRY_URL.into()),
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| DEFAULT_KAFKA_BROKERS.into()),
            http,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisteredSchema {
    pub id: u32,
    pub schema: String,
    pub subject: Option<String>,
}

#[derive(Debug)]
pub struct KafkaObservation {
    pub event: MovieEvent,
    pub key: String,
    pub partition: i32,
    pub offset: i64,
    pub schema_id: u32,
}

#[derive(Debug, Row, Deserialize)]
struct ChEvent {
    #[serde(with = "clickhouse::serde::uuid")]
    event_id: Uuid,
    user_id: String,
    movie_id: String,
    event_type: String,
    device_type: String,
    session_id: String,
    progress_seconds: i32,
}

pub struct KafkaObserver {
    consumer: BaseConsumer,
    partition_count: usize,
}

impl KafkaObserver {
    pub fn from_topic_end(kafka_brokers: &str) -> Result<Self> {
        let consumer = build_consumer(kafka_brokers)?;
        let metadata = consumer
            .fetch_metadata(Some(TOPIC), KAFKA_TIMEOUT)
            .context("failed to fetch Kafka topic metadata")?;
        let topic = metadata
            .topics()
            .iter()
            .find(|topic| topic.name() == TOPIC)
            .context("movie-events topic missing from Kafka metadata")?;

        ensure!(
            !topic.partitions().is_empty(),
            "movie-events topic has no partitions"
        );

        let mut assignments = TopicPartitionList::new();
        for partition in topic.partitions() {
            let (_, high_watermark) = consumer
                .fetch_watermarks(TOPIC, partition.id(), KAFKA_TIMEOUT)
                .with_context(|| {
                    format!(
                        "failed to fetch watermarks for {} partition {}",
                        TOPIC,
                        partition.id()
                    )
                })?;
            assignments
                .add_partition_offset(TOPIC, partition.id(), Offset::Offset(high_watermark))
                .with_context(|| {
                    format!(
                        "failed to assign {} partition {} at offset {}",
                        TOPIC,
                        partition.id(),
                        high_watermark
                    )
                })?;
        }

        consumer
            .assign(&assignments)
            .context("failed to assign Kafka observer to topic partitions")?;

        Ok(Self {
            consumer,
            partition_count: topic.partitions().len(),
        })
    }

    pub fn partition_count(&self) -> usize {
        self.partition_count
    }

    pub fn wait_for_events(
        &self,
        expected_ids: &[Uuid],
        timeout: Duration,
    ) -> Result<Vec<KafkaObservation>> {
        let schema = common::schema::parse_schema().context("failed to parse Avro schema")?;
        let mut found = HashMap::new();
        let expected: HashMap<Uuid, usize> = expected_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();
        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline && found.len() < expected.len() {
            match self.consumer.poll(POLL_INTERVAL) {
                Some(Ok(message)) => {
                    let observation = decode_message(&schema, message)
                        .context("failed to decode Kafka message")?;
                    if expected.contains_key(&observation.event.event_id) {
                        found.insert(observation.event.event_id, observation);
                    }
                }
                Some(Err(err)) => {
                    return Err(anyhow::anyhow!("Kafka poll failed: {err}"));
                }
                None => {}
            }
        }

        ensure!(
            found.len() == expected.len(),
            "observed {} Kafka messages, expected {}",
            found.len(),
            expected.len()
        );

        let mut ordered = Vec::with_capacity(expected_ids.len());
        for id in expected_ids {
            ordered.push(
                found.remove(id).with_context(|| {
                    format!("Kafka observer did not capture expected event {id}")
                })?,
            );
        }
        Ok(ordered)
    }
}

pub fn unique_string_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

pub fn build_event(
    user_id: &str,
    session_id: &str,
    event_type: EventType,
    timestamp: i64,
    progress_seconds: i32,
) -> MovieEvent {
    MovieEvent {
        event_id: Uuid::new_v4(),
        user_id: user_id.to_owned(),
        movie_id: "test-movie-001".into(),
        event_type,
        timestamp,
        device_type: DeviceType::Desktop,
        session_id: session_id.to_owned(),
        progress_seconds,
    }
}

pub async fn wait_for_registered_schema(
    env: &TestEnv,
    timeout: Duration,
) -> Result<RegisteredSchema> {
    let deadline = Instant::now() + timeout;
    let url = format!(
        "{}/subjects/{}/versions/latest",
        env.schema_registry_url, SCHEMA_SUBJECT
    );

    loop {
        let response = env.http.get(&url).send().await.with_context(|| {
            format!(
                "failed to reach Schema Registry at {}",
                env.schema_registry_url
            )
        })?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read Schema Registry response body")?;

        if status.is_success() {
            let schema: RegisteredSchema = serde_json::from_str(&body)
                .context("failed to parse Schema Registry latest-schema response")?;
            return Ok(schema);
        }

        if status == reqwest::StatusCode::NOT_FOUND && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(250)).await;
            continue;
        }

        return Err(anyhow::anyhow!(
            "Schema Registry returned {} for {}: {}",
            status,
            url,
            body
        ));
    }
}

pub fn assert_registered_schema_matches_code(schema: &RegisteredSchema) -> Result<()> {
    if let Some(subject) = &schema.subject {
        ensure!(
            subject == SCHEMA_SUBJECT,
            "unexpected schema subject: expected {}, got {}",
            SCHEMA_SUBJECT,
            subject
        );
    }

    let expected_schema: serde_json::Value =
        serde_json::from_str(AVRO_SCHEMA_STR).context("failed to parse local Avro schema JSON")?;
    let registered_schema: serde_json::Value = serde_json::from_str(&schema.schema)
        .context("failed to parse registered Avro schema JSON")?;

    ensure!(
        registered_schema == expected_schema,
        "registered schema does not match avro/movie_event.avsc"
    );
    Ok(())
}

pub async fn post_event(env: &TestEnv, event: &MovieEvent) -> Result<()> {
    let response = env
        .http
        .post(format!("{}/events", env.producer_url))
        .json(event)
        .send()
        .await
        .with_context(|| format!("failed to POST event {} to producer", event.event_id))?;

    ensure!(
        response.status().is_success(),
        "producer returned {} while publishing {}",
        response.status(),
        event.event_id
    );
    Ok(())
}

pub fn assert_messages_use_registered_schema(
    observed: &[KafkaObservation],
    schema_id: u32,
    expected_events: &[&MovieEvent],
) -> Result<()> {
    ensure!(
        observed.len() == expected_events.len(),
        "observed {} Kafka messages, expected {}",
        observed.len(),
        expected_events.len()
    );

    for (observation, expected) in observed.iter().zip(expected_events.iter()) {
        ensure!(
            observation.schema_id == schema_id,
            "event {} used schema id {}, expected {}",
            observation.event.event_id,
            observation.schema_id,
            schema_id
        );
        ensure!(
            observation.key == expected.user_id,
            "event {} used Kafka key {:?}, expected {:?}",
            observation.event.event_id,
            observation.key,
            expected.user_id
        );
        assert_event_matches(&observation.event, expected).with_context(|| {
            format!(
                "Kafka payload for {} did not match the published event",
                expected.event_id
            )
        })?;
    }

    Ok(())
}

pub fn assert_same_user_partitioning(observed: &[KafkaObservation]) -> Result<()> {
    ensure!(
        observed.len() >= 2,
        "need at least two Kafka messages to verify partitioning"
    );

    let first = &observed[0];
    for observation in &observed[1..] {
        ensure!(
            observation.partition == first.partition,
            "same user landed in different partitions: {} and {}",
            first.partition,
            observation.partition
        );
        ensure!(
            observation.offset > first.offset,
            "same-partition offsets are not increasing: {} then {}",
            first.offset,
            observation.offset
        );
    }

    Ok(())
}

pub async fn wait_for_clickhouse_event(
    env: &TestEnv,
    expected: &MovieEvent,
    timeout: Duration,
) -> Result<()> {
    let clickhouse = clickhouse::Client::default().with_url(&env.clickhouse_url);
    let deadline = Instant::now() + timeout;
    let event_id = expected.event_id.to_string();

    loop {
        let result = clickhouse
            .query(
                "SELECT event_id, user_id, movie_id, event_type, device_type, session_id, progress_seconds \
                 FROM movie_events WHERE event_id = ?",
            )
            .bind(event_id.as_str())
            .fetch_optional::<ChEvent>()
            .await;

        match result {
            Ok(Some(actual)) => {
                ensure!(
                    actual.event_id == expected.event_id,
                    "ClickHouse returned event_id {} for expected {}",
                    actual.event_id,
                    expected.event_id
                );
                ensure!(
                    actual.user_id == expected.user_id,
                    "ClickHouse user_id mismatch for {}",
                    expected.event_id
                );
                ensure!(
                    actual.movie_id == expected.movie_id,
                    "ClickHouse movie_id mismatch for {}",
                    expected.event_id
                );
                ensure!(
                    actual.event_type == event_type_name(&expected.event_type),
                    "ClickHouse event_type mismatch for {}",
                    expected.event_id
                );
                ensure!(
                    actual.device_type == device_type_name(&expected.device_type),
                    "ClickHouse device_type mismatch for {}",
                    expected.event_id
                );
                ensure!(
                    actual.session_id == expected.session_id,
                    "ClickHouse session_id mismatch for {}",
                    expected.event_id
                );
                ensure!(
                    actual.progress_seconds == expected.progress_seconds,
                    "ClickHouse progress_seconds mismatch for {}",
                    expected.event_id
                );
                return Ok(());
            }
            Ok(None) => {}
            Err(err) if Instant::now() < deadline => {
                eprintln!("ClickHouse query error (will retry): {}", err);
            }
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "ClickHouse query failed while waiting for {}: {}",
                    expected.event_id,
                    err
                ));
            }
        }

        ensure!(
            Instant::now() < deadline,
            "event {} not found in ClickHouse within {}s",
            expected.event_id,
            timeout.as_secs()
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn build_consumer(kafka_brokers: &str) -> Result<BaseConsumer> {
    let group_id = format!("integration-tests-{}", Uuid::new_v4());
    ClientConfig::new()
        .set("bootstrap.servers", kafka_brokers)
        .set("group.id", &group_id)
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest")
        .create()
        .context("failed to create Kafka consumer for integration tests")
}

fn decode_message(
    schema: &apache_avro::Schema,
    message: common::rdkafka::message::BorrowedMessage<'_>,
) -> Result<KafkaObservation> {
    let key = String::from_utf8(
        message
            .key()
            .context("Kafka message was missing a key")?
            .to_vec(),
    )
    .context("Kafka message key was not valid UTF-8")?;

    let payload = message
        .payload()
        .context("Kafka message was missing a payload")?;
    ensure!(
        payload.len() >= 5,
        "Kafka payload too short for Confluent wire format"
    );
    ensure!(
        payload[0] == 0,
        "unexpected Confluent wire-format magic byte {}",
        payload[0]
    );

    let schema_id = u32::from_be_bytes(
        payload[1..5]
            .try_into()
            .context("failed to read schema id from Kafka payload")?,
    );
    let mut avro_payload = &payload[5..];
    let value = apache_avro::from_avro_datum(schema, &mut avro_payload, None)
        .context("failed to decode Avro payload from Kafka")?;
    let event = apache_avro::from_value::<MovieEvent>(&value)
        .context("failed to deserialize Avro value into MovieEvent")?;

    Ok(KafkaObservation {
        event,
        key,
        partition: message.partition(),
        offset: message.offset(),
        schema_id,
    })
}

fn assert_event_matches(actual: &MovieEvent, expected: &MovieEvent) -> Result<()> {
    ensure!(
        actual.event_id == expected.event_id,
        "event_id mismatch: expected {}, got {}",
        expected.event_id,
        actual.event_id
    );
    ensure!(
        actual.user_id == expected.user_id,
        "user_id mismatch: expected {:?}, got {:?}",
        expected.user_id,
        actual.user_id
    );
    ensure!(
        actual.movie_id == expected.movie_id,
        "movie_id mismatch: expected {:?}, got {:?}",
        expected.movie_id,
        actual.movie_id
    );
    ensure!(
        event_type_name(&actual.event_type) == event_type_name(&expected.event_type),
        "event_type mismatch: expected {}, got {}",
        event_type_name(&expected.event_type),
        event_type_name(&actual.event_type)
    );
    ensure!(
        actual.timestamp == expected.timestamp,
        "timestamp mismatch: expected {}, got {}",
        expected.timestamp,
        actual.timestamp
    );
    ensure!(
        device_type_name(&actual.device_type) == device_type_name(&expected.device_type),
        "device_type mismatch: expected {}, got {}",
        device_type_name(&expected.device_type),
        device_type_name(&actual.device_type)
    );
    ensure!(
        actual.session_id == expected.session_id,
        "session_id mismatch: expected {:?}, got {:?}",
        expected.session_id,
        actual.session_id
    );
    ensure!(
        actual.progress_seconds == expected.progress_seconds,
        "progress_seconds mismatch: expected {}, got {}",
        expected.progress_seconds,
        actual.progress_seconds
    );
    Ok(())
}

fn event_type_name(event_type: &EventType) -> &'static str {
    match event_type {
        EventType::ViewStarted => "VIEW_STARTED",
        EventType::ViewFinished => "VIEW_FINISHED",
        EventType::ViewPaused => "VIEW_PAUSED",
        EventType::ViewResumed => "VIEW_RESUMED",
        EventType::Liked => "LIKED",
        EventType::Searched => "SEARCHED",
    }
}

fn device_type_name(device_type: &DeviceType) -> &'static str {
    match device_type {
        DeviceType::Mobile => "MOBILE",
        DeviceType::Desktop => "DESKTOP",
        DeviceType::Tv => "TV",
        DeviceType::Tablet => "TABLET",
    }
}
