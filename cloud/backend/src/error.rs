//! The common error body of the REST API (03 §5.2 rule 6).

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// One field-level problem of a validation error.
#[derive(Debug, Clone, Serialize)]
pub struct FieldProblem {
    /// Name of the offending field, e.g. `password`.
    pub field: &'static str,
    /// What is wrong, without echoing the value.
    pub problem: &'static str,
}

/// An error answered to a REST client.
///
/// The message never contains personal or health data; internal causes are
/// logged with the trace id and replaced by a generic message.
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status.
    pub status: StatusCode,
    /// Stable machine-readable code, e.g. `validation_failed`.
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Per-field problems for validation errors.
    pub details: Vec<FieldProblem>,
    /// Seconds to wait, for `429 Too Many Requests`.
    pub retry_after_s: Option<u64>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(rename = "traceId")]
    trace_id: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    details: &'a [FieldProblem],
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: Vec::new(),
            retry_after_s: None,
        }
    }

    /// Builds a `400` for one invalid field.
    #[must_use]
    pub fn invalid(field: &'static str, problem: &'static str) -> Self {
        let mut err = Self::new(
            StatusCode::BAD_REQUEST,
            "validation_failed",
            "the request breaks a validation rule",
        );
        err.details.push(FieldProblem { field, problem });
        err
    }

    /// Builds a `401` for missing, expired or invalid credentials.
    #[must_use]
    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing or invalid credentials",
        )
    }

    /// Builds a `403` for a collection the caller's role may not use.
    #[must_use]
    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "this role may not use this resource",
        )
    }

    /// Builds a `404`, also used when the caller may not see the object.
    #[must_use]
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "no such object")
    }

    /// Builds a `409` for an object that already exists.
    #[must_use]
    pub fn conflict(what: &'static str) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "conflict",
            format!("{what} already exists"),
        )
    }

    /// Builds a `413` for a body over the size limit.
    #[must_use]
    pub fn payload_too_large() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "the request body is too large",
        )
    }

    /// Builds a `429` with a `Retry-After` header.
    #[must_use]
    pub fn rate_limited(retry_after_s: u64) -> Self {
        let mut err = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many requests",
        );
        err.retry_after_s = Some(retry_after_s);
        err
    }

    /// Builds a `500` for an unexpected failure; the cause is logged, not returned.
    #[must_use]
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "an internal error occurred",
        )
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "database error");
        Self::internal()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let trace_id = uuid::Uuid::new_v4().simple().to_string();
        if self.status.is_server_error() {
            tracing::error!(trace_id, code = self.code, "request failed");
        }
        let body = ErrorBody {
            code: self.code,
            message: &self.message,
            trace_id,
            details: &self.details,
        };
        let mut response = (self.status, Json(body)).into_response();
        if let Some(secs) = self.retry_after_s
            && let Ok(value) = HeaderValue::from_str(&secs.to_string())
        {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        response
    }
}

/// Result type of REST handlers.
pub type ApiResult<T> = Result<T, ApiError>;
