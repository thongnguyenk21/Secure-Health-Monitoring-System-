//! `GET /live`: WebSocket of alert changes for the clinician web app (D30).

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use super::extract::ApiQuery;
use crate::domain::Role;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Query of `GET /live`.
#[derive(Debug, Deserialize)]
pub struct LiveQuery {
    /// Access token; browsers cannot set headers on a WebSocket.
    pub access_token: String,
}

/// Upgrades to a WebSocket that forwards every alert change.
///
/// The socket closes when the token expires; the client reconnects with a
/// fresh token.
///
/// # Errors
/// `401` for an invalid token, `403` for patients (they get push instead).
pub async fn open(
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<LiveQuery>,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let claims = state
        .jwt
        .verify(&q.access_token)
        .map_err(|_| ApiError::unauthorized())?;
    if claims.tenant_id != state.tenant_id {
        return Err(ApiError::unauthorized());
    }
    if !matches!(claims.role, Role::Clinician | Role::Admin) {
        return Err(ApiError::forbidden());
    }
    let ttl_s = u64::try_from(claims.exp - chrono::Utc::now().timestamp()).unwrap_or(0);
    Ok(ws.on_upgrade(move |socket| forward(socket, state, ttl_s)))
}

async fn forward(mut socket: WebSocket, state: AppState, ttl_s: u64) {
    let mut rx = state.live.subscribe();
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(ttl_s));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            () = &mut deadline => break,
            event = rx.recv() => match event {
                Ok(e) => {
                    let Ok(text) = serde_json::to_string(&e) else { continue };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                // A slow client skipped events; it refetches on the next one.
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}
