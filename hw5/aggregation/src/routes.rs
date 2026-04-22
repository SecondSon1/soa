use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::{export_for_date, run_aggregation_for_date};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/aggregate", post(post_aggregate))
        .route("/export", post(post_export))
        .route("/health", get(health))
        .with_state(state)
}

#[derive(Deserialize)]
struct DateRequest {
    date: NaiveDate,
}

async fn post_aggregate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DateRequest>,
) -> impl IntoResponse {
    match run_aggregation_for_date(&state, req.date).await {
        Ok(results) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "date": req.date,
                "metrics_computed": results.len(),
                "results": results,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{:#}", e)})),
        ),
    }
}

async fn post_export(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DateRequest>,
) -> impl IntoResponse {
    match export_for_date(&state, req.date).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"date": req.date, "status": "exported"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{:#}", e)})),
        ),
    }
}

async fn health() -> StatusCode {
    StatusCode::OK
}
