//! REST routes of `cloud-api.openapi.yaml` 0.2.0 and the live WebSocket.

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::error::ApiError;
use crate::state::AppState;

pub mod alerts;
pub mod audit;
pub mod auth;
pub mod consents;
pub mod devices;
pub mod extract;
pub mod health;
pub mod live;
pub mod measurements;
pub mod models;
pub mod patients;
pub mod system;
pub mod users;

/// Largest JSON body accepted, bytes (07 §3.6).
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// Builds the router with every route and layer.
///
/// # Arguments
/// * `state` - shared state.
/// * `cors_origin` - comma-separated origins allowed by CORS in development, e.g.
///   `http://localhost:5173,http://localhost:19006` (web app and Expo web preview).
pub fn router(state: AppState, cors_origin: Option<&str>) -> Router {
    let v1 = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/password", post(auth::change_password))
        .route("/auth/register", post(auth::register))
        .route("/me", get(users::me))
        .route("/users", get(users::list))
        .route("/devices", get(devices::list).post(devices::create))
        .route(
            "/devices/{device_id}",
            get(devices::get_one).patch(devices::update),
        )
        .route("/devices/{device_id}/assign", post(devices::assign))
        .route("/devices/{device_id}/revoke", post(devices::revoke))
        .route("/measurements", get(measurements::list))
        .route("/measurements/latest", get(measurements::latest))
        .route("/alerts", get(alerts::list))
        .route("/alerts/{alert_id}/ack", post(alerts::acknowledge))
        .route("/patients", get(patients::list))
        .route("/patients/{user_id}", get(patients::get_one))
        .route(
            "/patients/{user_id}/notes",
            get(patients::list_notes).post(patients::create_note),
        )
        .route("/consents", get(consents::list).post(consents::record))
        .route("/models", get(models::list))
        .route("/models/{version}/install", post(models::install))
        .route("/models/{version}/rollback", post(models::rollback))
        .route("/audit", get(audit::list))
        .route("/system/status", get(system::status))
        .route("/live", get(live::open));

    let mut app = Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", v1)
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        // Span on the path only: the query of /live carries the access token,
        // which must never reach a log (07 §4.3).
        .layer(TraceLayer::new_for_http().make_span_with(
            |req: &axum::http::Request<axum::body::Body>| {
                tracing::info_span!("http", method = %req.method(), path = %req.uri().path())
            },
        ))
        .with_state(state);

    let origins: Vec<HeaderValue> = cors_origin
        .unwrap_or_default()
        .split(',')
        .filter_map(|o| HeaderValue::from_str(o.trim()).ok())
        .filter(|o| !o.is_empty())
        .collect();
    if !origins.is_empty() {
        app = app.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::PUT,
                    Method::DELETE,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        );
    }
    app
}

async fn not_found() -> Response {
    ApiError::not_found().into_response()
}

/// Adds `Cache-Control: no-store` to a response carrying health data.
pub fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
