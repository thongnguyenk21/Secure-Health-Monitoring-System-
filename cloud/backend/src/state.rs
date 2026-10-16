//! Shared state handed to every request handler.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::alerts::rules::Thresholds;
use crate::alerts::{AlertEngine, LiveEvent};
use crate::infra::audit::AuditLog;
use crate::infra::jwt::JwtKeys;
use crate::infra::limiter::RateLimiter;

/// Everything a handler needs; cheap to clone.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Database pool.
    pub db: PgPool,
    /// Access token keys.
    pub jwt: Arc<JwtKeys>,
    /// Tenant of this deployment.
    pub tenant_id: Uuid,
    /// Audit chain writer.
    pub audit: Arc<AuditLog>,
    /// Alert engine.
    pub engine: Arc<AlertEngine>,
    /// Live alert stream for WebSocket clients.
    pub live: broadcast::Sender<LiveEvent>,
    /// Login attempts per client IP: 5 per 5 minutes (07 §3.6).
    pub login_limiter: Arc<RateLimiter<IpAddr>>,
    /// Precomputed hash checked when the email is unknown.
    pub dummy_hash: Arc<str>,
}

impl AppState {
    /// Builds the state.
    ///
    /// # Arguments
    /// * `db` - connected pool with migrations applied.
    /// * `jwt` - access token keys.
    /// * `tenant_id` - tenant of this deployment.
    ///
    /// # Errors
    /// Returns the hashing error of the timing-equaliser hash; it does not
    /// happen with the default parameters.
    pub fn new(
        db: PgPool,
        jwt: JwtKeys,
        tenant_id: Uuid,
    ) -> Result<Self, argon2::password_hash::Error> {
        let (live, _) = broadcast::channel(256);
        Ok(Self {
            engine: Arc::new(AlertEngine::new(Thresholds::default(), live.clone())),
            db,
            jwt: Arc::new(jwt),
            tenant_id,
            audit: Arc::new(AuditLog::default()),
            live,
            login_limiter: Arc::new(RateLimiter::new(5, Duration::from_secs(300))),
            dummy_hash: crate::infra::password::dummy_hash()?.into(),
        })
    }
}
