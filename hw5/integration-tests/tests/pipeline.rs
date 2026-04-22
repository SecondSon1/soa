use anyhow::Result;
use chrono::Utc;
use common::event::EventType;
use integration_tests::{
    KafkaObserver, TestEnv, assert_messages_use_registered_schema,
    assert_registered_schema_matches_code, assert_same_user_partitioning, build_event, post_event,
    unique_string_id, wait_for_clickhouse_event, wait_for_registered_schema,
};
use std::time::Duration;

#[tokio::test]
async fn pipeline_uses_schema_registry_partitions_by_user_and_reaches_clickhouse() -> Result<()> {
    let env = TestEnv::from_env()?;
    let registered_schema = wait_for_registered_schema(&env, Duration::from_secs(10)).await?;
    assert_registered_schema_matches_code(&registered_schema)?;

    let kafka = KafkaObserver::from_topic_end(&env.kafka_brokers)?;
    assert_eq!(
        kafka.partition_count(),
        3,
        "movie-events topic should have exactly 3 partitions"
    );

    let user_id = unique_string_id("integration-user");
    let session_id = unique_string_id("integration-session");
    let base_timestamp = Utc::now().timestamp_millis();
    let started = build_event(
        &user_id,
        &session_id,
        EventType::ViewStarted,
        base_timestamp,
        0,
    );
    let paused = build_event(
        &user_id,
        &session_id,
        EventType::ViewPaused,
        base_timestamp + 1_000,
        120,
    );

    post_event(&env, &started).await?;
    post_event(&env, &paused).await?;

    let kafka_messages = kafka.wait_for_events(
        &[started.event_id, paused.event_id],
        Duration::from_secs(10),
    )?;
    assert_messages_use_registered_schema(
        &kafka_messages,
        registered_schema.id,
        &[&started, &paused],
    )?;
    assert_same_user_partitioning(&kafka_messages)?;

    wait_for_clickhouse_event(&env, &started, Duration::from_secs(30)).await?;
    wait_for_clickhouse_event(&env, &paused, Duration::from_secs(30)).await?;
    Ok(())
}
