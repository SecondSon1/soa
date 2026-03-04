use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::{
  HeaderMap, HeaderValue, Method, StatusCode,
  header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderName},
};
use axum::middleware::Next;
use axum::response::Response;
use chrono::{SecondsFormat, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use super::auth::AuthenticatedUserId;

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

#[derive(Serialize)]
struct ValidationErrorDetail {
  field: String,
  violation: String,
}

#[derive(Serialize)]
struct ValidationErrorResponse {
  error_code: &'static str,
  message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  details: Option<Value>,
}

pub(super) async fn api_logging_middleware(mut request: Request, next: Next) -> Response {
  let request_id = Uuid::new_v4().to_string();
  let method = request.method().clone();
  let endpoint = request.uri().path().to_owned();
  let request_user_id = extract_user_id(request.headers());

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
  response = normalize_validation_error_response(response).await;
  let user_id = response
    .extensions()
    .get::<AuthenticatedUserId>()
    .map(|value| value.0.clone())
    .or(request_user_id);
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

async fn normalize_validation_error_response(response: Response) -> Response {
  if response.status() != StatusCode::BAD_REQUEST && response.status() != StatusCode::UNPROCESSABLE_ENTITY {
    return response;
  }

  let (mut parts, body) = response.into_parts();
  let body_bytes = match to_bytes(body, usize::MAX).await {
    Ok(bytes) => bytes,
    Err(_) => return Response::from_parts(parts, Body::empty()),
  };

  if body_bytes.is_empty() {
    return Response::from_parts(parts, Body::from(body_bytes));
  }

  let body_text = match serde_json::from_slice::<Value>(&body_bytes) {
    Ok(json) => {
      if is_validation_error_payload(&json) {
        parts.status = StatusCode::BAD_REQUEST;
        return Response::from_parts(parts, Body::from(body_bytes));
      }
      extract_error_text_from_json(&json).unwrap_or_else(|| json.to_string())
    }
    Err(_) => match std::str::from_utf8(&body_bytes) {
      Ok(text) => text.to_owned(),
      Err(_) => return Response::from_parts(parts, Body::from(body_bytes)),
    },
  };

  let details = extract_validation_details(&body_text);
  let validation_error = ValidationErrorResponse {
    error_code: "VALIDATION_ERROR",
    message: "Request validation failed".to_owned(),
    details: Some(serde_json::json!({ "errors": details })),
  };

  let json_bytes = match serde_json::to_vec(&validation_error) {
    Ok(value) => value,
    Err(_) => return Response::from_parts(parts, Body::from(body_bytes)),
  };

  parts.status = StatusCode::BAD_REQUEST;
  parts
    .headers
    .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
  parts.headers.remove(CONTENT_LENGTH);
  Response::from_parts(parts, Body::from(json_bytes))
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

fn is_validation_error_payload(json: &Value) -> bool {
  let Some(obj) = json.as_object() else {
    return false;
  };

  let code_ok = obj
    .get("error_code")
    .and_then(Value::as_str)
    .map(|code| code == "VALIDATION_ERROR")
    .unwrap_or(false);

  let message_ok = obj.get("message").map(Value::is_string).unwrap_or(false);
  code_ok && message_ok
}

fn extract_error_text_from_json(json: &Value) -> Option<String> {
  match json {
    Value::String(value) => Some(value.clone()),
    Value::Object(obj) => {
      for key in ["message", "error", "detail", "title"] {
        if let Some(value) = obj.get(key).and_then(Value::as_str) {
          return Some(value.to_owned());
        }
      }
      None
    }
    _ => None,
  }
}

fn extract_validation_details(body_text: &str) -> Vec<ValidationErrorDetail> {
  lazy_static! {
    static ref VALIDATION_DETAIL_RE: Regex =
      Regex::new(r"([^:]+): Validation error: ([^\[]+)").expect("validation regex must compile");
    static ref MISSING_FIELD_RE: Regex =
      Regex::new(r"missing field `([^`]+)`").expect("missing-field regex must compile");
    static ref UNKNOWN_FIELD_RE: Regex =
      Regex::new(r"unknown field `([^`]+)`").expect("unknown-field regex must compile");
  }

  let mut details = Vec::new();

  details.extend(VALIDATION_DETAIL_RE.captures_iter(body_text).filter_map(|captures| {
    let field = captures.get(1)?.as_str().to_owned();
    let violation = captures.get(2)?.as_str().trim().to_owned();
    Some(ValidationErrorDetail { field, violation })
  }));

  if details.is_empty() {
    if let Some(captures) = MISSING_FIELD_RE.captures(body_text) {
      if let Some(field) = captures.get(1) {
        details.push(ValidationErrorDetail {
          field: field.as_str().to_owned(),
          violation: "missing field".to_owned(),
        });
      }
    }
  }

  if details.is_empty() {
    if let Some(captures) = UNKNOWN_FIELD_RE.captures(body_text) {
      if let Some(field) = captures.get(1) {
        details.push(ValidationErrorDetail {
          field: field.as_str().to_owned(),
          violation: "unknown field".to_owned(),
        });
      }
    }
  }

  if details.is_empty() {
    details.push(ValidationErrorDetail {
      field: "request".to_owned(),
      violation: body_text.trim().to_owned(),
    });
  }

  details
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

#[cfg(test)]
mod tests {
  use axum::body::to_bytes;
  use axum::http::StatusCode;
  use axum::response::Response;
  use serde_json::Value;

  use super::normalize_validation_error_response;

  #[tokio::test]
  async fn normalizes_plain_text_missing_field_error() {
    let input = Response::builder()
      .status(StatusCode::UNPROCESSABLE_ENTITY)
      .body(axum::body::Body::from(
        "Failed to deserialize the JSON body into the target type: missing field `price` at line 5 column 3",
      ))
      .expect("response must build");

    let response = normalize_validation_error_response(input).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX)
      .await
      .expect("response body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("response must be json");

    assert_eq!(json["error_code"], "VALIDATION_ERROR");
    assert_eq!(json["message"], "Request validation failed");
    assert_eq!(json["details"]["errors"][0]["field"], "price");
    assert_eq!(json["details"]["errors"][0]["violation"], "missing field");
  }

  #[tokio::test]
  async fn normalizes_json_wrapped_deserialize_error() {
    let input = Response::builder()
      .status(StatusCode::BAD_REQUEST)
      .header(axum::http::header::CONTENT_TYPE, "application/json")
      .body(axum::body::Body::from(
        r#"{"message":"Failed to deserialize the JSON body into the target type: missing field `price` at line 5 column 3"}"#,
      ))
      .expect("response must build");

    let response = normalize_validation_error_response(input).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX)
      .await
      .expect("response body must be readable");
    let json: Value = serde_json::from_slice(&body).expect("response must be json");

    assert_eq!(json["error_code"], "VALIDATION_ERROR");
    assert_eq!(json["message"], "Request validation failed");
    assert_eq!(json["details"]["errors"][0]["field"], "price");
    assert_eq!(json["details"]["errors"][0]["violation"], "missing field");
  }
}
