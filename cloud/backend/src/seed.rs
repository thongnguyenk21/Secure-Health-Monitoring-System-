//! Development seed data: an admin, a clinician, patients and their wearables.
//!
//! Never run against a real deployment: the accounts share one known password.

use sqlx::PgPool;
use uuid::Uuid;

use crate::infra::password;

/// Password of every seeded account (development only).
pub const DEV_PASSWORD: &str = "iomt-dev-password";

/// Patients created by the seed: (email, subject code, display name).
pub const DEV_PATIENTS: [(&str, &str, &str); 6] = [
    ("bn017@iomt.local", "BN-017", "Nam, 54"),
    ("bn031@iomt.local", "BN-031", "Nữ, 71"),
    ("bn009@iomt.local", "BN-009", "Nam, 66"),
    ("bn021@iomt.local", "BN-021", "Nữ, 59"),
    ("bn004@iomt.local", "BN-004", "Nam, 48"),
    ("bn025@iomt.local", "BN-025", "Nữ, 62"),
];

/// Why seeding failed.
#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    /// Database error.
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    /// Hashing error.
    #[error("password hashing failed")]
    Hash,
    /// A device id is not 32 lowercase hex characters.
    #[error("device id {0} is not 32 lowercase hex characters")]
    BadDeviceId(usize),
}

async fn upsert_user(
    db: &PgPool,
    phc: &str,
    email: &str,
    role: &str,
    display: Option<&str>,
    code: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO users (id, email, password_hash, role, display_name, subject_code)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (email) DO UPDATE SET display_name = EXCLUDED.display_name
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(phc)
    .bind(role)
    .bind(display)
    .bind(code)
    .fetch_one(db)
    .await
}

/// Creates the development accounts and assigns `device_ids` to patients in order.
///
/// Idempotent: existing accounts and devices are kept.
///
/// # Arguments
/// * `db` - migrated database.
/// * `device_ids` - wearable ids, e.g. from `device-sim ids`.
///
/// # Errors
/// Returns [`SeedError`] for a bad device id or a database failure.
pub async fn seed_dev(db: &PgPool, device_ids: &[String]) -> Result<(), SeedError> {
    if let Some(bad) = device_ids
        .iter()
        .find(|d| iomt_protocol::DeviceId::new(d.as_str()).is_err())
    {
        return Err(SeedError::BadDeviceId(bad.len()));
    }
    let phc = tokio::task::spawn_blocking(|| password::hash(DEV_PASSWORD))
        .await
        .map_err(|_| SeedError::Hash)?
        .map_err(|_| SeedError::Hash)?;
    upsert_user(
        db,
        &phc,
        "admin@iomt.local",
        "admin",
        Some("Quản trị viên"),
        None,
    )
    .await?;
    upsert_user(
        db,
        &phc,
        "bs.minh@iomt.local",
        "clinician",
        Some("BS. Minh"),
        None,
    )
    .await?;
    let mut patients = Vec::new();
    for (email, code, name) in DEV_PATIENTS {
        patients.push(upsert_user(db, &phc, email, "patient", Some(name), Some(code)).await?);
    }
    // Data of seeded patients is collected, so they start with the monitoring
    // consent on record; training consent stays off until they choose it.
    for patient in &patients {
        sqlx::query(
            "INSERT INTO consents (id, subject_user_id, scope, granted, form_version, recorded_by)
             SELECT $1, $2, 'monitoring', TRUE, '1.0', $2
             WHERE NOT EXISTS (SELECT 1 FROM consents WHERE subject_user_id = $2 AND scope = 'monitoring')",
        )
        .bind(Uuid::new_v4())
        .bind(patient)
        .execute(db)
        .await?;
    }
    for (device, patient) in device_ids.iter().zip(patients.iter()) {
        sqlx::query(
            "INSERT INTO devices (device_id, kind, label, assigned_user_id) VALUES ($1, 'wearable', $2, $3)
             ON CONFLICT (device_id) DO UPDATE SET assigned_user_id = EXCLUDED.assigned_user_id",
        )
        .bind(device)
        .bind(format!("Đồng hồ {}", &device[..4]))
        .bind(patient)
        .execute(db)
        .await?;
    }
    sqlx::query(
        "INSERT INTO devices (device_id, kind, status, label) VALUES ($1, 'gateway', 'active', 'Pi phòng 204')
         ON CONFLICT (device_id) DO NOTHING",
    )
    .bind("0000000000000000000000000000a204")
    .execute(db)
    .await?;
    Ok(())
}
