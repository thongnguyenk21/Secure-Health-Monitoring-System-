//! Rotating refresh tokens (REQ-SEC-010).
//!
//! A login starts a family. Each refresh marks the presented token used and
//! issues the next one in the same family. Presenting a token that was already
//! used means it leaked, so the whole family is revoked.

use chrono::{Duration, Utc};
use ring::rand::SecureRandom as _;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Lifetime of a refresh token, days.
pub const REFRESH_TTL_DAYS: i64 = 30;

/// A new refresh token for the client.
#[derive(Debug, Clone)]
pub struct IssuedRefresh {
    /// Opaque token, 64 hex characters (256 bits).
    pub token: String,
    /// Family (session) id.
    pub family_id: Uuid,
    /// Expiry, UTC epoch milliseconds.
    pub expires_at_ms: i64,
}

/// Outcome of presenting a refresh token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rotation {
    /// Valid: the caller gets a new token in the same family.
    Rotated {
        /// Owner of the family.
        user_id: Uuid,
        /// Family id, also the session id in access tokens.
        family_id: Uuid,
    },
    /// Unknown, expired or revoked token.
    Invalid,
    /// A used token was presented again; the family is now revoked.
    Reused,
}

fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn random_token() -> Result<String, sqlx::Error> {
    let mut bytes = [0u8; 32];
    ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| sqlx::Error::Protocol("random generator unavailable".into()))?;
    Ok(hex::encode(bytes))
}

/// Stores a new token in `family_id` for `user_id`.
///
/// # Errors
/// Returns the database error, or a protocol error if the OS random generator fails.
pub async fn issue(
    db: &PgPool,
    user_id: Uuid,
    family_id: Uuid,
) -> Result<IssuedRefresh, sqlx::Error> {
    let token = random_token()?;
    let expires = Utc::now() + Duration::days(REFRESH_TTL_DAYS);
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, family_id, token_hash, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(family_id)
    .bind(hash(&token))
    .bind(expires)
    .execute(db)
    .await?;
    Ok(IssuedRefresh {
        token,
        family_id,
        expires_at_ms: expires.timestamp_millis(),
    })
}

/// Marks a presented token used, or revokes its family if it was used before.
///
/// # Errors
/// Returns the database error.
pub async fn rotate(db: &PgPool, token: &str) -> Result<Rotation, sqlx::Error> {
    let mut tx = db.begin().await?;
    let row: Option<(Uuid, Uuid, bool, bool, bool)> = sqlx::query_as(
        "SELECT user_id, family_id, used_at IS NOT NULL, revoked_at IS NOT NULL, expires_at < now()
         FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE",
    )
    .bind(hash(token))
    .fetch_optional(&mut *tx)
    .await?;
    let outcome = match row {
        None => Rotation::Invalid,
        Some((_, _, _, true, _) | (_, _, false, false, true)) => Rotation::Invalid,
        Some((_, family_id, true, false, _)) => {
            revoke_family_tx(&mut tx, family_id).await?;
            Rotation::Reused
        }
        Some((user_id, family_id, false, false, false)) => {
            sqlx::query("UPDATE refresh_tokens SET used_at = now() WHERE token_hash = $1")
                .bind(hash(token))
                .execute(&mut *tx)
                .await?;
            Rotation::Rotated { user_id, family_id }
        }
    };
    tx.commit().await?;
    Ok(outcome)
}

async fn revoke_family_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    family_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Revokes the family of a token (logout).
///
/// # Returns
/// The owner of the token, or `None` if the token is unknown.
///
/// # Errors
/// Returns the database error.
pub async fn revoke_by_token(db: &PgPool, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let row: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_id, family_id FROM refresh_tokens WHERE token_hash = $1")
            .bind(hash(token))
            .fetch_optional(db)
            .await?;
    if let Some((_, family_id)) = row {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL",
        )
        .bind(family_id)
        .execute(db)
        .await?;
    }
    Ok(row.map(|(user, _)| user))
}

/// Revokes every family of a user (password change).
///
/// # Errors
/// Returns the database error.
pub async fn revoke_all_for_user(db: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}
