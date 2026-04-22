CREATE TABLE IF NOT EXISTS movie_events_kafka (
    event_id UUID,
    user_id String,
    movie_id String,
    event_type String,
    timestamp Int64,
    device_type String,
    session_id String,
    progress_seconds Int32
) ENGINE = Kafka
SETTINGS
    kafka_broker_list = 'kafka1:29092,kafka2:29092',
    kafka_topic_list = 'movie-events',
    kafka_group_name = 'clickhouse-consumer',
    kafka_format = 'AvroConfluent',
    format_avro_schema_registry_url = 'http://schema-registry:8081';

CREATE TABLE IF NOT EXISTS movie_events (
    event_id UUID,
    user_id String,
    movie_id String,
    event_type String,
    timestamp DateTime64(3, 'UTC'),
    device_type String,
    session_id String,
    progress_seconds Int32
) ENGINE = MergeTree
PARTITION BY toYYYYMM(timestamp)
ORDER BY (user_id, timestamp);

CREATE MATERIALIZED VIEW IF NOT EXISTS movie_events_mv TO movie_events AS
SELECT
    event_id,
    user_id,
    movie_id,
    event_type,
    fromUnixTimestamp64Milli(timestamp) AS timestamp,
    device_type,
    session_id,
    progress_seconds
FROM movie_events_kafka;

CREATE TABLE IF NOT EXISTS daily_metrics (
    date Date,
    metric_name String,
    dimension String DEFAULT '',
    value Float64,
    computed_at DateTime64(3, 'UTC')
) ENGINE = ReplacingMergeTree(computed_at)
ORDER BY (date, metric_name, dimension);
