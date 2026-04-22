use anyhow::Result;
use chrono::{Duration, Utc};
use common::event::{DeviceType, EventType, MovieEvent};
use common::kafka::publish_event;
use rand::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

const MOVIE_IDS: &[&str] = &[
    "movie-001",
    "movie-002",
    "movie-003",
    "movie-004",
    "movie-005",
    "movie-006",
    "movie-007",
    "movie-008",
    "movie-009",
    "movie-010",
    "movie-011",
    "movie-012",
    "movie-013",
    "movie-014",
    "movie-015",
    "movie-016",
    "movie-017",
    "movie-018",
    "movie-019",
    "movie-020",
];

const DEVICES: &[DeviceType] = &[
    DeviceType::Mobile,
    DeviceType::Desktop,
    DeviceType::Tv,
    DeviceType::Tablet,
];

pub async fn generate_events(
    state: Arc<AppState>,
    user_count: usize,
    days_back: usize,
) -> Result<()> {
    let mut rng = StdRng::from_os_rng();
    let users: Vec<String> = (0..user_count)
        .map(|i| format!("user-{:04}", i + 1))
        .collect();

    // Assign user behavior patterns for retention simulation:
    // ~15% churned (only appear on first day or two)
    // ~15% power users (appear almost every day)
    // ~70% regular users (appear on some days randomly)
    let churned_count = user_count * 15 / 100;
    let power_count = user_count * 15 / 100;

    let now = Utc::now();
    let mut total = 0u64;

    for day_offset in (0..days_back).rev() {
        let day_start = now - Duration::days(day_offset as i64);
        let day_start_ms = day_start
            .date_naive()
            .and_hms_opt(8, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();

        for (i, user_id) in users.iter().enumerate() {
            let is_churned = i < churned_count;
            let is_power = i >= churned_count && i < churned_count + power_count;

            let active = if is_churned {
                day_offset >= days_back - 2
            } else if is_power {
                rng.random_bool(0.9)
            } else {
                rng.random_bool(0.5)
            };

            if !active {
                continue;
            }

            let sessions = rng.random_range(1..=3);
            for s in 0..sessions {
                let session_id = format!(
                    "session-{}-{}-{}",
                    day_start.format("%Y%m%d"),
                    user_id,
                    s + 1
                );
                let device = DEVICES.choose(&mut rng).unwrap().clone();
                let movie = *MOVIE_IDS.choose(&mut rng).unwrap();
                let session_start_ms =
                    day_start_ms + (s as i64) * 3_600_000 + rng.random_range(0..3_600_000);

                let movie_duration_secs = rng.random_range(3600..=7200); // 1-2 hours
                let mut progress = 0i32;
                let mut ts = session_start_ms;

                // VIEW_STARTED
                let event = MovieEvent {
                    event_id: Uuid::new_v4(),
                    user_id: user_id.clone(),
                    movie_id: movie.to_string(),
                    event_type: EventType::ViewStarted,
                    timestamp: ts,
                    device_type: device.clone(),
                    session_id: session_id.clone(),
                    progress_seconds: progress,
                };
                publish_event(&state.producer, &state.schema, state.schema_id, &event).await?;
                total += 1;

                // Maybe pause/resume cycle
                if rng.random_bool(0.4) {
                    progress += rng.random_range(300..=1200);
                    ts += rng.random_range(300_000..=1_200_000);
                    let pause = MovieEvent {
                        event_id: Uuid::new_v4(),
                        user_id: user_id.clone(),
                        movie_id: movie.to_string(),
                        event_type: EventType::ViewPaused,
                        timestamp: ts,
                        device_type: device.clone(),
                        session_id: session_id.clone(),
                        progress_seconds: progress,
                    };
                    publish_event(&state.producer, &state.schema, state.schema_id, &pause).await?;
                    total += 1;

                    ts += rng.random_range(60_000..=600_000);
                    let resume = MovieEvent {
                        event_id: Uuid::new_v4(),
                        user_id: user_id.clone(),
                        movie_id: movie.to_string(),
                        event_type: EventType::ViewResumed,
                        timestamp: ts,
                        device_type: device.clone(),
                        session_id: session_id.clone(),
                        progress_seconds: progress,
                    };
                    publish_event(&state.producer, &state.schema, state.schema_id, &resume).await?;
                    total += 1;
                }

                // VIEW_FINISHED or partial (70% finish)
                if rng.random_bool(0.7) {
                    progress = movie_duration_secs;
                    ts += rng.random_range(1_800_000..=7_200_000);
                    let finish = MovieEvent {
                        event_id: Uuid::new_v4(),
                        user_id: user_id.clone(),
                        movie_id: movie.to_string(),
                        event_type: EventType::ViewFinished,
                        timestamp: ts,
                        device_type: device.clone(),
                        session_id: session_id.clone(),
                        progress_seconds: progress,
                    };
                    publish_event(&state.producer, &state.schema, state.schema_id, &finish).await?;
                    total += 1;

                    // Maybe like after finishing
                    if rng.random_bool(0.3) {
                        ts += rng.random_range(1_000..=30_000);
                        let like = MovieEvent {
                            event_id: Uuid::new_v4(),
                            user_id: user_id.clone(),
                            movie_id: movie.to_string(),
                            event_type: EventType::Liked,
                            timestamp: ts,
                            device_type: device.clone(),
                            session_id: session_id.clone(),
                            progress_seconds: progress,
                        };
                        publish_event(&state.producer, &state.schema, state.schema_id, &like)
                            .await?;
                        total += 1;
                    }
                }

                // Occasional search
                if rng.random_bool(0.2) {
                    ts += rng.random_range(10_000..=120_000);
                    let search = MovieEvent {
                        event_id: Uuid::new_v4(),
                        user_id: user_id.clone(),
                        movie_id: movie.to_string(),
                        event_type: EventType::Searched,
                        timestamp: ts,
                        device_type: device.clone(),
                        session_id: session_id.clone(),
                        progress_seconds: 0,
                    };
                    publish_event(&state.producer, &state.schema, state.schema_id, &search).await?;
                    total += 1;
                }
            }
        }

        tracing::info!(day_offset, total, "generated events for day");
    }

    tracing::info!(total, "event generation complete");
    Ok(())
}
