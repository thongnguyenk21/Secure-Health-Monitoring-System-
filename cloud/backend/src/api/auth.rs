//! Login, token refresh, logout, password change and user registration.

use std::net::SocketAddr;
use std::time::Instant;

use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiJson, Caller};
use super::users::{USER_COLUMNS, UserDto, UserRow};
use crate::domain::Role;
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::infra::password::{self, MAX_LEN, MIN_LEN};
use crate::infra::tokens::{self, Rotation};
use crate::state::AppState;

/// Body of `POST /auth/login`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    /// Account email.
    pub email: String,
    /// Clear password; never logged.
    pub password: String,
}

/// Body of `POST /auth/refresh` and `POST /auth/logout`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    /// Opaque refresh token.
    pub refresh_token: String,
}

/// Access and refresh tokens.
#[derive(Debug, Serialize)]
pub struct TokenPair {
    /// JWT for the `Authorization` header.
    pub access_token: String,
    /// Access token expiry, epoch ms.
    pub access_expires_at_unix_ms: i64,
    /// Rotating refresh token.
    pub refresh_token: String,
    /// Refresh token expiry, epoch ms.
    pub refresh_expires_at_unix_ms: i64,
}

/// Body of `POST /auth/password`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordChange {
    /// Current password.
    pub current_password: String,
    /// New password, 12..=256 characters.
    pub new_password: String,
}

/// Body of `POST /auth/register`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterUser {
    /// Account email.
    pub email: String,
    /// Initial password, 12..=256 characters.
    pub password: String,
    /// Role.
    pub role: Role,
    /// Name shown in the apps.
    pub display_name: Option<String>,
    /// Patient code, e.g. `BN-017`; patients only.
    pub subject_code: Option<String>,
}

fn check_password(field: &'static str, pw: &str) -> ApiResult<()> {
    if pw.chars().count() < MIN_LEN || pw.len() > MAX_LEN {
        return Err(ApiError::invalid(field, "must be 12 to 256 characters"));
    }
    Ok(())
}

fn normalise_email(email: &str) -> ApiResult<String> {
    let e = email.trim().to_lowercase();
    let valid = e.len() <= 256
        && e.split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if valid {
        Ok(e)
    } else {
        Err(ApiError::invalid("email", "not a valid email address"))
    }
}

/// Checks a subject code such as `BN-017`.
#[must_use]
pub fn subject_code_ok(code: &str) -> bool {
    let Some((prefix, digits)) = code.split_once('-') else {
        return false;
    };
    prefix.len() == 2
        && prefix.bytes().all(|b| b.is_ascii_uppercase())
        && (3..=6).contains(&digits.len())
        && digits.bytes().all(|b| b.is_ascii_digit())
}

async fn hash_password(pw: String) -> ApiResult<String> {
    tokio::task::spawn_blocking(move || password::hash(&pw))
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(|_| ApiError::internal())
}

async fn verify_password(pw: String, phc: String) -> ApiResult<bool> {
    tokio::task::spawn_blocking(move || password::verify(&pw, &phc))
        .await
        .map_err(|_| ApiError::internal())
}

async fn issue_pair(
    state: &AppState,
    user: Uuid,
    role: Role,
    family: Uuid,
) -> ApiResult<TokenPair> {
    let now_s = chrono::Utc::now().timestamp();
    let (access_token, exp_s) = state
        .jwt
        .issue(user, state.tenant_id, role, family, now_s)
        .map_err(|_| ApiError::internal())?;
    let refresh = tokens::issue(&state.db, user, family).await?;
    Ok(TokenPair {
        access_token,
        access_expires_at_unix_ms: exp_s * 1000,
        refresh_token: refresh.token,
        refresh_expires_at_unix_ms: refresh.expires_at_ms,
    })
}

/// `POST /auth/login`.
///
/// The rate limit is checked before the password hash (REQ-SEC-014), and an
/// unknown email costs the same hashing time as a known one.
///
/// # Errors
/// `400` for a malformed body, `401` for bad credentials, `429` over the limit.
pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    ApiJson(body): ApiJson<LoginRequest>,
) -> ApiResult<Json<TokenPair>> {
    state
        .login_limiter
        .check(&peer.ip(), Instant::now())
        .map_err(ApiError::rate_limited)?;
    let email = normalise_email(&body.email)?;
    let row: Option<(Uuid, String, String, bool)> =
        sqlx::query_as("SELECT id, password_hash, role, disabled FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;
    let phc = row
        .as_ref()
        .map_or_else(|| state.dummy_hash.to_string(), |r| r.1.clone());
    let ok = verify_password(body.password, phc).await?;
    let Some((id, _, role, _)) = row.filter(|r| ok && !r.3) else {
        state
            .audit
            .record(&state.db, None, "auth.login", "", Outcome::Denied)
            .await;
        return Err(ApiError::unauthorized());
    };
    let role = Role::parse(&role).ok_or_else(ApiError::internal)?;
    let pair = issue_pair(&state, id, role, Uuid::new_v4()).await?;
    state
        .audit
        .record(&state.db, Some(id), "auth.login", "", Outcome::Allowed)
        .await;
    Ok(Json(pair))
}

/// `POST /auth/refresh`: rotates the refresh token.
///
/// # Errors
/// `401` for an unknown, expired, revoked or reused token.
pub async fn refresh(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<RefreshRequest>,
) -> ApiResult<Json<TokenPair>> {
    match tokens::rotate(&state.db, &body.refresh_token).await? {
        Rotation::Rotated { user_id, family_id } => {
            let row: Option<(String, bool)> =
                sqlx::query_as("SELECT role, disabled FROM users WHERE id = $1")
                    .bind(user_id)
                    .fetch_optional(&state.db)
                    .await?;
            let (role, _) = row.filter(|r| !r.1).ok_or_else(ApiError::unauthorized)?;
            let role = Role::parse(&role).ok_or_else(ApiError::internal)?;
            Ok(Json(issue_pair(&state, user_id, role, family_id).await?))
        }
        Rotation::Reused => {
            state
                .audit
                .record(&state.db, None, "auth.refresh_reuse", "", Outcome::Denied)
                .await;
            Err(ApiError::unauthorized())
        }
        Rotation::Invalid => Err(ApiError::unauthorized()),
    }
}

/// `POST /auth/logout`: revokes the session's refresh family.
///
/// # Errors
/// `401` without a valid access token.
pub async fn logout(
    State(state): State<AppState>,
    caller: Caller,
    ApiJson(body): ApiJson<RefreshRequest>,
) -> ApiResult<StatusCode> {
    tokens::revoke_by_token(&state.db, &body.refresh_token).await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "auth.logout",
            "",
            Outcome::Allowed,
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /auth/password`: changes the caller's password and revokes all sessions.
///
/// # Errors
/// `400` for a short password, `401` for a wrong current password.
pub async fn change_password(
    State(state): State<AppState>,
    caller: Caller,
    ApiJson(body): ApiJson<PasswordChange>,
) -> ApiResult<StatusCode> {
    check_password("new_password", &body.new_password)?;
    let phc: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(caller.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    if !verify_password(body.current_password, phc).await? {
        return Err(ApiError::unauthorized());
    }
    let new_hash = hash_password(body.new_password).await?;
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(caller.id)
        .bind(new_hash)
        .execute(&state.db)
        .await?;
    tokens::revoke_all_for_user(&state.db, caller.id).await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "auth.password",
            "",
            Outcome::Allowed,
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /auth/register`: creates a user (admin).
///
/// # Errors
/// `400` for invalid fields, `403` for a non-admin, `409` for a taken email or code.
pub async fn register(
    State(state): State<AppState>,
    caller: Caller,
    ApiJson(body): ApiJson<RegisterUser>,
) -> ApiResult<impl IntoResponse> {
    if caller.require(&[Role::Admin]).is_err() {
        state
            .audit
            .record(
                &state.db,
                Some(caller.id),
                "user.create",
                "",
                Outcome::Denied,
            )
            .await;
        return Err(ApiError::forbidden());
    }
    let email = normalise_email(&body.email)?;
    check_password("password", &body.password)?;
    let display_name = body
        .display_name
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty());
    if display_name
        .as_ref()
        .is_some_and(|n| n.chars().count() > 64)
    {
        return Err(ApiError::invalid("display_name", "at most 64 characters"));
    }
    if let Some(code) = &body.subject_code {
        if body.role != Role::Patient {
            return Err(ApiError::invalid(
                "subject_code",
                "only patients have a subject code",
            ));
        }
        if !subject_code_ok(code) {
            return Err(ApiError::invalid("subject_code", "must look like BN-017"));
        }
    }
    let hash = hash_password(body.password).await?;
    let id = Uuid::new_v4();
    let row: Result<UserRow, sqlx::Error> = sqlx::query_as(&format!(
        "INSERT INTO users (id, email, password_hash, role, display_name, subject_code)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {USER_COLUMNS}"
    ))
    .bind(id)
    .bind(&email)
    .bind(hash)
    .bind(body.role.as_str())
    .bind(display_name)
    .bind(&body.subject_code)
    .fetch_one(&state.db)
    .await;
    let row = match row {
        Ok(r) => r,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(ApiError::conflict("a user with this email or subject code"));
        }
        Err(e) => return Err(e.into()),
    };
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "user.create",
            &id.to_string(),
            Outcome::Allowed,
        )
        .await;
    let dto: UserDto = row.try_into()?;
    Ok((StatusCode::CREATED, Json(dto)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_codes_follow_the_pattern() {
        assert!(subject_code_ok("BN-017"));
        assert!(subject_code_ok("BN-123456"));
        assert!(!subject_code_ok("bn-017"));
        assert!(!subject_code_ok("BN-01"));
        assert!(!subject_code_ok("BN017"));
    }

    #[test]
    fn emails_are_normalised_and_checked() {
        assert_eq!(
            normalise_email(" BS.Minh@IoMT.local ").unwrap(),
            "bs.minh@iomt.local"
        );
        assert!(normalise_email("no-at-sign").is_err());
        assert!(normalise_email("@iomt.local").is_err());
    }

    #[test]
    fn short_passwords_are_refused() {
        assert!(check_password("password", "short").is_err());
        assert!(check_password("password", "long enough password").is_ok());
    }
}
