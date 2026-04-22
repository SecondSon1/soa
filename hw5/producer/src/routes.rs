use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;

use common::event::MovieEvent;
use common::kafka::publish_event;

use crate::AppState;
use crate::generator;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/events", post(post_event))
        .route("/generate", post(post_generate))
        .route("/health", get(health))
        .with_state(state)
}

async fn post_event(
    State(state): State<Arc<AppState>>,
    Json(event): Json<MovieEvent>,
) -> impl IntoResponse {
    match publish_event(&state.producer, &state.schema, state.schema_id, &event).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"event_id": event.event_id})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{:#}", e)})),
        ),
    }
}

#[derive(Deserialize)]
struct GenerateRequest {
    #[serde(default = "default_user_count")]
    user_count: usize,
    #[serde(default = "default_days_back")]
    days_back: usize,
}

fn default_user_count() -> usize {
    50
}
fn default_days_back() -> usize {
    9
}

async fn post_generate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateRequest>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(e) = generator::generate_events(state, req.user_count, req.days_back).await {
            tracing::error!("event generation failed: {:#}", e);
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status": "generation_started"})),
    )
}

async fn health() -> StatusCode {
    StatusCode::OK
}
