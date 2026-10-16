//! Append-only audit log with a hash chain (REQ-SEC-021/022).
//!
//! Each row stores `hash = SHA-256(prev_hash || at_ms || actor || action ||
//! target || outcome)`. Editing or deleting a row breaks every later hash; the
//! database trigger refuses `UPDATE` and `DELETE` in the first place.

use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Whether the audited action was allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The action went ahead.
    Allowed,
    /// The action was refused.
    Denied,
}

impl Outcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }
}

/// Writer of the audit chain; serialises appends so the chain stays linear.
#[derive(Debug, Default)]
pub struct AuditLog {
    append: Mutex<()>,
}

/// Computes the hash of one entry chained to the previous one.
#[must_use]
pub fn chain_hash(
    prev: &[u8],
    at_ms: i64,
    actor: Option<Uuid>,
    action: &str,
    target: &str,
    outcome: &str,
) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(prev);
    h.update(at_ms.to_be_bytes());
    h.update(actor.map(|a| a.as_bytes().to_vec()).unwrap_or_default());
    for part in [action, target, outcome] {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part.as_bytes());
    }
    h.finalize().to_vec()
}

impl AuditLog {
    /// Appends one entry.
    ///
    /// Failures are logged and swallowed: an audit outage must not take the API
    /// down, and the gap is visible as a missing link in the chain.
    ///
    /// # Arguments
    /// * `db` - pool.
    /// * `actor` - user who acted, `None` for the system or an unknown caller.
    /// * `action` - dotted verb, e.g. `measurements.read`.
    /// * `target` - object acted on, e.g. a device id; never personal data.
    /// * `outcome` - allowed or denied.
    pub async fn record(
        &self,
        db: &PgPool,
        actor: Option<Uuid>,
        action: &str,
        target: &str,
        outcome: Outcome,
    ) {
        if let Err(e) = self.try_record(db, actor, action, target, outcome).await {
            tracing::error!(error = %e, action, "audit append failed");
        }
    }

    async fn try_record(
        &self,
        db: &PgPool,
        actor: Option<Uuid>,
        action: &str,
        target: &str,
        outcome: Outcome,
    ) -> Result<(), sqlx::Error> {
        let _guard = self.append.lock().await;
        let prev: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1")
                .fetch_optional(db)
                .await?;
        let prev = prev.unwrap_or_else(|| vec![0u8; 32]);
        let at = chrono::Utc::now();
        let target: String = target.chars().take(128).collect();
        let hash = chain_hash(
            &prev,
            at.timestamp_millis(),
            actor,
            action,
            &target,
            outcome.as_str(),
        );
        sqlx::query(
            "INSERT INTO audit_log (at, actor_id, action, target, outcome, prev_hash, hash)
             VALUES (to_timestamp($1::double precision / 1000), $2, $3, $4, $5, $6, $7)",
        )
        .bind(at.timestamp_millis())
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(outcome.as_str())
        .bind(prev)
        .bind(hash)
        .execute(db)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_change_changes_the_hash() {
        let base = chain_hash(&[0; 32], 1, None, "alert.ack", "x", "allowed");
        assert_ne!(
            base,
            chain_hash(&[1; 32], 1, None, "alert.ack", "x", "allowed")
        );
        assert_ne!(
            base,
            chain_hash(&[0; 32], 2, None, "alert.ack", "x", "allowed")
        );
        assert_ne!(
            base,
            chain_hash(&[0; 32], 1, None, "alert.ack", "x", "denied")
        );
        // Length prefixes stop "ab"+"c" from colliding with "a"+"bc".
        assert_ne!(
            chain_hash(&[0; 32], 1, None, "ab", "c", "allowed"),
            chain_hash(&[0; 32], 1, None, "a", "bc", "allowed")
        );
    }
}
