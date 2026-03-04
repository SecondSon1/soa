use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, Method, header::HeaderName};
use axum::middleware::Next;
use axum::response::Response;
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

const X_REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
const X_USER_ID_HEADER: HeaderName = HeaderName::from_static("x-user-id");

#[derive(Serialize)]
struct ApiRequestLog {
  log_type: &'static str,
  request_id: String,
  method: String,
  endpoint: String,
  status_code: u16,
  duration_ms: u128,
  user_id: Option<String>,
  timestamp: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  request_body: Option<Value>,
}

pub(super) async fn api_logging_middleware(mut request: Request, next: Next) -> Response {
  let request_id = Uuid::new_v4().to_string();
  let method = request.method().clone();
  let endpoint = request.uri().path().to_owned();
  let user_id = extract_user_id(request.headers());

  let request_body = if is_mutating_method(&method) {
    let (parts, body) = request.into_parts();

    match to_bytes(body, usize::MAX).await {
      Ok(bytes) => {
        let masked_body = mask_request_body(bytes.as_ref());
        request = Request::from_parts(parts, Body::from(bytes));
        Some(masked_body)
      }
      Err(error) => {
        request = Request::from_parts(parts, Body::empty());
        Some(Value::String(format!("<failed_to_read_body: {error}>")))
      }
    }
  } else {
    None
  };

  let start = Instant::now();
  let mut response = next.run(request).await;
  let duration_ms = start.elapsed().as_millis();
  let status_code = response.status().as_u16();

  if let Ok(value) = HeaderValue::from_str(&request_id) {
    response.headers_mut().insert(X_REQUEST_ID_HEADER.clone(), value);
  }

  let log_line = ApiRequestLog {
    log_type: "api_request",
    request_id,
    method: method.as_str().to_owned(),
    endpoint,
    status_code,
    duration_ms,
    user_id,
    timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    request_body,
  };

  emit_json_log(&log_line);
  response
}

fn extract_user_id(headers: &HeaderMap) -> Option<String> {
  headers
    .get(X_USER_ID_HEADER)
    .and_then(|value| value.to_str().ok())
    .map(ToOwned::to_owned)
}

fn is_mutating_method(method: &Method) -> bool {
  matches!(*method, Method::POST | Method::PUT | Method::DELETE)
}

fn mask_request_body(bytes: &[u8]) -> Value {
  if bytes.is_empty() {
    return Value::Null;
  }

  match serde_json::from_slice::<Value>(bytes) {
    Ok(mut json) => {
      mask_sensitive_fields(&mut json);
      json
    }
    Err(_) => match std::str::from_utf8(bytes) {
      Ok(text) => Value::String(text.to_owned()),
      Err(_) => Value::String("<non_utf8_body>".to_owned()),
    },
  }
}

fn mask_sensitive_fields(value: &mut Value) {
  match value {
    Value::Object(map) => {
      for (key, nested) in map.iter_mut() {
        if is_sensitive_key(key) {
          *nested = Value::String("***".to_owned());
        } else {
          mask_sensitive_fields(nested);
        }
      }
    }
    Value::Array(values) => {
      for nested in values.iter_mut() {
        mask_sensitive_fields(nested);
      }
    }
    _ => {}
  }
}

fn is_sensitive_key(key: &str) -> bool {
  matches!(
    key.to_ascii_lowercase().as_str(),
    "password" | "passwd" | "pwd" | "secret" | "token" | "access_token" | "refresh_token" | "api_key" | "authorization"
  )
}

fn emit_json_log(log_line: &ApiRequestLog) {
  match serde_json::to_string(log_line) {
    Ok(line) => println!("{line}"),
    Err(error) => eprintln!(
      "{{\"log_type\":\"api_request\",\"request_id\":\"{}\",\"error\":\"failed_to_serialize_api_log: {}\"}}",
      log_line.request_id, error
    ),
  }
}
