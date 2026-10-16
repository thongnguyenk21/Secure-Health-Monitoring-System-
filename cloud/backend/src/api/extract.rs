//! Request extractors: the authenticated caller and the paging parameters.

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::Role;
use crate::error::ApiError;
use crate::state::AppState;

/// The caller, from a valid bearer access token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caller {
    /// User id.
    pub id: Uuid,
    /// Role.
    pub role: Role,
    /// Session (refresh family) id.
    pub sid: Uuid,
}

impl Caller {
    /// Fails with `403` unless the caller has one of `roles`.
    ///
    /// # Errors
    /// Returns [`ApiError::forbidden`].
    pub fn require(&self, roles: &[Role]) -> Result<(), ApiError> {
        if roles.contains(&self.role) {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

impl FromRequestParts<AppState> for Caller {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(ApiError::unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(ApiError::unauthorized)?;
        let claims = state
            .jwt
            .verify(token.trim())
            .map_err(|_| ApiError::unauthorized())?;
        if claims.tenant_id != state.tenant_id {
            return Err(ApiError::unauthorized());
        }
        Ok(Self {
            id: claims.sub,
            role: claims.role,
            sid: claims.sid,
        })
    }
}

/// `cursor` and `limit` query parameters of paged routes.
///
/// Query strings are text; when this struct is `#[serde(flatten)]`ed into
/// another query, serde hands every value over as a string, so `limit` accepts
/// both a number and its decimal text.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageParams {
    /// Opaque cursor from the previous page.
    pub cursor: Option<String>,
    /// Page size, 1..=1000, default 100.
    #[serde(default, deserialize_with = "number_or_text")]
    pub limit: Option<i64>,
}

fn number_or_text<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(i64),
        Text(String),
    }
    match Option::<Raw>::deserialize(d)? {
        None => Ok(None),
        Some(Raw::Number(n)) => Ok(Some(n)),
        Some(Raw::Text(t)) => t.parse().map(Some).map_err(serde::de::Error::custom),
    }
}

/// `Query` that answers a malformed query string with the common error body.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiQuery<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|q| Self(q.0))
            .map_err(|_| ApiError::invalid("query", "missing or malformed query parameter"))
    }
}

/// `Json` body that answers a malformed body with the common error body.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiJson<T>(pub T);

impl<T, S> axum::extract::FromRequest<S> for ApiJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::Json::<T>::from_request(req, state)
            .await
            .map(|j| Self(j.0))
            .map_err(|e| match e {
                axum::extract::rejection::JsonRejection::BytesRejection(_) => {
                    ApiError::payload_too_large()
                }
                _ => ApiError::invalid("body", "missing, malformed or with unknown fields"),
            })
    }
}

impl PageParams {
    /// Returns the validated page size.
    ///
    /// # Errors
    /// Returns a `400` when `limit` is outside 1..=1000.
    pub fn limit(&self) -> Result<i64, ApiError> {
        match self.limit {
            None => Ok(100),
            Some(n) if (1..=1000).contains(&n) => Ok(n),
            Some(_) => Err(ApiError::invalid("limit", "must be between 1 and 1000")),
        }
    }

    /// Decodes the cursor text.
    ///
    /// # Errors
    /// Returns a `400` when the cursor is not one this server issued.
    pub fn cursor(&self) -> Result<Option<String>, ApiError> {
        self.cursor
            .as_deref()
            .map(|c| {
                URL_SAFE_NO_PAD
                    .decode(c)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .ok_or_else(|| ApiError::invalid("cursor", "not a valid cursor"))
            })
            .transpose()
    }
}

/// Encodes a cursor for the next page.
#[must_use]
pub fn encode_cursor(text: &str) -> String {
    URL_SAFE_NO_PAD.encode(text.as_bytes())
}

/// Splits a `"<i64>|<uuid>"` cursor.
///
/// # Errors
/// Returns a `400` for any other shape.
pub fn split_cursor(text: &str) -> Result<(i64, Uuid), ApiError> {
    let bad = || ApiError::invalid("cursor", "not a valid cursor");
    let (a, b) = text.split_once('|').ok_or_else(bad)?;
    Ok((
        a.parse().map_err(|_| bad())?,
        Uuid::parse_str(b).map_err(|_| bad())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors_round_trip() {
        let id = Uuid::new_v4();
        let text = format!("42|{id}");
        let page = PageParams {
            cursor: Some(encode_cursor(&text)),
            limit: None,
        };
        let decoded = page.cursor().unwrap().unwrap();
        assert_eq!(split_cursor(&decoded).unwrap(), (42, id));
        assert_eq!(page.limit().unwrap(), 100);
    }

    #[test]
    fn bad_paging_is_a_400() {
        let page = PageParams {
            cursor: Some("%%%".into()),
            limit: Some(5000),
        };
        assert!(page.cursor().is_err());
        assert!(page.limit().is_err());
        assert!(split_cursor("no-separator").is_err());
    }
}
