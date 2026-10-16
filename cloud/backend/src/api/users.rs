//! `GET /users` (admin) and the user row shared with registration.

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiQuery, Caller, PageParams, encode_cursor, split_cursor};
use crate::domain::Role;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::time::{from_ms, to_ms};

/// Columns selected for a user.
pub const USER_COLUMNS: &str = "id, email, role, display_name, subject_code, created_at";

/// One row of `users` without the password hash.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    /// User id.
    pub id: Uuid,
    /// Email.
    pub email: String,
    /// Stored role spelling.
    pub role: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Patient code.
    pub subject_code: Option<String>,
    /// Creation time.
    pub created_at: DateTime<Utc>,
}

/// A user as the API returns it (OpenAPI `User`).
#[derive(Debug, Clone, Serialize)]
pub struct UserDto {
    /// User id.
    pub id: Uuid,
    /// Email.
    pub email: String,
    /// Role.
    pub role: Role,
    /// Display name.
    pub display_name: Option<String>,
    /// Patient code.
    pub subject_code: Option<String>,
    /// Creation time, epoch ms.
    pub created_at_unix_ms: i64,
}

impl TryFrom<UserRow> for UserDto {
    type Error = ApiError;
    fn try_from(r: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: r.id,
            email: r.email,
            role: Role::parse(&r.role).ok_or_else(ApiError::internal)?,
            display_name: r.display_name,
            subject_code: r.subject_code,
            created_at_unix_ms: to_ms(r.created_at),
        })
    }
}

/// One page of users.
#[derive(Debug, Serialize)]
pub struct UserPage {
    /// Users, oldest first.
    pub items: Vec<UserDto>,
    /// Cursor of the next page.
    pub next_cursor: Option<String>,
}

/// Query of `GET /users`.
#[derive(Debug, Deserialize)]
pub struct UserQuery {
    /// Filter by role.
    pub role: Option<Role>,
    /// Paging.
    #[serde(flatten)]
    pub page: PageParams,
}

/// `GET /users`: lists accounts (admin).
///
/// # Errors
/// `403` for non-admins, `400` for bad paging.
pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
    ApiQuery(q): ApiQuery<UserQuery>,
) -> ApiResult<Json<UserPage>> {
    caller.require(&[Role::Admin])?;
    let limit = q.page.limit()?;
    let after = q.page.cursor()?.map(|c| split_cursor(&c)).transpose()?;
    let (after_ts, after_id) = match after {
        Some((ms, id)) => (from_ms(ms), Some(id)),
        None => (None, None),
    };
    let rows: Vec<UserRow> = sqlx::query_as(&format!(
        "SELECT {USER_COLUMNS} FROM users
         WHERE ($1::text IS NULL OR role = $1)
           AND ($2::timestamptz IS NULL OR (created_at, id) > ($2, $3))
         ORDER BY created_at, id LIMIT $4"
    ))
    .bind(q.role.map(Role::as_str))
    .bind(after_ts)
    .bind(after_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let more = rows.len() as i64 > limit;
    let items: Vec<UserDto> = rows
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(100))
        .map(UserDto::try_from)
        .collect::<Result<_, _>>()?;
    let next_cursor = more
        .then(|| {
            items
                .last()
                .map(|u| encode_cursor(&format!("{}|{}", u.created_at_unix_ms, u.id)))
        })
        .flatten();
    Ok(Json(UserPage { items, next_cursor }))
}

/// `GET /me`: the caller's own account.
///
/// # Errors
/// `401` when the account no longer exists.
pub async fn me(State(state): State<AppState>, caller: Caller) -> ApiResult<Json<UserDto>> {
    let row: Option<UserRow> = sqlx::query_as(&format!(
        "SELECT {USER_COLUMNS} FROM users WHERE id = $1 AND NOT disabled"
    ))
    .bind(caller.id)
    .fetch_optional(&state.db)
    .await?;
    Ok(Json(row.ok_or_else(ApiError::unauthorized)?.try_into()?))
}
