//! Process configuration read from environment variables.

use std::net::SocketAddr;

use uuid::Uuid;

/// Why the configuration could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required variable is not set.
    #[error("environment variable {0} is required")]
    Missing(&'static str),
    /// A variable is set but cannot be parsed.
    #[error("environment variable {name} is invalid: {reason}")]
    Invalid {
        /// Variable name.
        name: &'static str,
        /// What is wrong with it; never the value itself, which may be a secret.
        reason: String,
    },
}

/// Settings of one backend process.
#[derive(Debug, Clone)]
pub struct Config {
    /// PostgreSQL URL with TimescaleDB (`DATABASE_URL`).
    pub database_url: String,
    /// Address of the REST API and WebSocket (`HTTP_ADDR`, default `0.0.0.0:8080`).
    pub http_addr: SocketAddr,
    /// Address of the gRPC ingest service (`GRPC_ADDR`, default `0.0.0.0:9443`).
    pub grpc_addr: SocketAddr,
    /// Ed25519 signing key for access tokens, PKCS#8 DER in base64
    /// (`JWT_ED25519_PKCS8_B64`). When unset a fresh key is generated at start,
    /// so tokens do not survive a restart (development only).
    pub jwt_key_pkcs8_b64: Option<String>,
    /// Origins allowed by CORS, comma-separated, e.g.
    /// `http://localhost:5173,http://localhost:19006` (`CORS_ORIGIN`).
    /// Unset means same-origin only (production behind Caddy).
    pub cors_origin: Option<String>,
    /// Tenant written into access tokens (`TENANT_ID`); one tenant per deployment in the MVP.
    pub tenant_id: Uuid,
}

/// Tenant used when `TENANT_ID` is not set.
pub const DEFAULT_TENANT: Uuid = Uuid::from_u128(1);

impl Config {
    /// Reads the configuration from the environment.
    ///
    /// # Errors
    /// Returns [`ConfigError`] when `DATABASE_URL` is missing or an address or
    /// id cannot be parsed.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;
        Ok(Self {
            database_url,
            http_addr: addr("HTTP_ADDR", "0.0.0.0:8080")?,
            grpc_addr: addr("GRPC_ADDR", "0.0.0.0:9443")?,
            jwt_key_pkcs8_b64: non_empty("JWT_ED25519_PKCS8_B64"),
            cors_origin: non_empty("CORS_ORIGIN"),
            tenant_id: match non_empty("TENANT_ID") {
                Some(raw) => Uuid::parse_str(&raw).map_err(|e| ConfigError::Invalid {
                    name: "TENANT_ID",
                    reason: e.to_string(),
                })?,
                None => DEFAULT_TENANT,
            },
        })
    }
}

fn non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn addr(name: &'static str, default: &str) -> Result<SocketAddr, ConfigError> {
    non_empty(name)
        .as_deref()
        .unwrap_or(default)
        .parse()
        .map_err(|e: std::net::AddrParseError| ConfigError::Invalid {
            name,
            reason: e.to_string(),
        })
}
