//! Access tokens: short-lived JWTs signed with Ed25519 (ADR-0015).
//!
//! Claims carry ids and the role only, never personal data. Tokens live
//! 15 minutes; revocation happens through the refresh token family.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use ring::signature::{Ed25519KeyPair, KeyPair as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::Role;

/// Lifetime of an access token, seconds.
pub const ACCESS_TTL_S: i64 = 15 * 60;

/// Why a signing key could not be loaded or created.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// The base64 text is not valid.
    #[error("the signing key is not valid base64")]
    Base64,
    /// The bytes are not an Ed25519 PKCS#8 document.
    #[error("the signing key is not an Ed25519 PKCS#8 key")]
    Pkcs8,
}

/// Claims of an access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// User id.
    pub sub: Uuid,
    /// Tenant of the user.
    pub tenant_id: Uuid,
    /// Role at the time of login.
    pub role: Role,
    /// Session (refresh token family) the token was issued for.
    pub sid: Uuid,
    /// Unique token id.
    pub jti: Uuid,
    /// Expiry, seconds since the Unix epoch.
    pub exp: i64,
    /// Issue time, seconds since the Unix epoch.
    pub iat: i64,
}

/// Signing and verification keys of the access tokens.
pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    kid: String,
}

impl std::fmt::Debug for JwtKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtKeys")
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
}

impl JwtKeys {
    /// Loads a key from base64 PKCS#8, or generates a fresh one.
    ///
    /// # Arguments
    /// * `pkcs8_b64` - Ed25519 private key as PKCS#8 DER in base64; `None` generates one.
    ///
    /// # Errors
    /// Returns [`KeyError`] if the given key cannot be decoded.
    pub fn load_or_generate(pkcs8_b64: Option<&str>) -> Result<Self, KeyError> {
        let pkcs8 = match pkcs8_b64 {
            Some(text) => STANDARD.decode(text.trim()).map_err(|_| KeyError::Base64)?,
            None => {
                let rng = ring::rand::SystemRandom::new();
                Ed25519KeyPair::generate_pkcs8(&rng)
                    .map_err(|_| KeyError::Pkcs8)?
                    .as_ref()
                    .to_vec()
            }
        };
        let pair = Ed25519KeyPair::from_pkcs8(&pkcs8).map_err(|_| KeyError::Pkcs8)?;
        let public = pair.public_key().as_ref();
        let kid = hex::encode(sha2_prefix(public));
        Ok(Self {
            encoding: EncodingKey::from_ed_der(&pkcs8),
            decoding: DecodingKey::from_ed_der(public),
            kid,
        })
    }

    /// Issues an access token.
    ///
    /// # Arguments
    /// * `user` - user id.
    /// * `tenant_id` - tenant of the user.
    /// * `role` - the user's role.
    /// * `sid` - session id (refresh family).
    /// * `now_s` - current time, seconds since the Unix epoch.
    ///
    /// # Returns
    /// The token and its expiry in seconds since the Unix epoch.
    ///
    /// # Errors
    /// Returns the signing error; it does not happen with a valid key.
    pub fn issue(
        &self,
        user: Uuid,
        tenant_id: Uuid,
        role: Role,
        sid: Uuid,
        now_s: i64,
    ) -> Result<(String, i64), jsonwebtoken::errors::Error> {
        let claims = Claims {
            sub: user,
            tenant_id,
            role,
            sid,
            jti: Uuid::new_v4(),
            exp: now_s + ACCESS_TTL_S,
            iat: now_s,
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.kid.clone());
        let token = jsonwebtoken::encode(&header, &claims, &self.encoding)?;
        Ok((token, claims.exp))
    }

    /// Verifies a token's signature and expiry.
    ///
    /// # Errors
    /// Returns the verification error for a bad signature, a wrong algorithm or
    /// an expired token.
    pub fn verify(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.leeway = 5;
        validation.set_required_spec_claims(&["exp", "sub"]);
        jsonwebtoken::decode::<Claims>(token, &self.decoding, &validation).map(|d| d.claims)
    }
}

fn sha2_prefix(bytes: &[u8]) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> JwtKeys {
        JwtKeys::load_or_generate(None).unwrap()
    }

    #[test]
    fn issued_tokens_verify() {
        let k = keys();
        let user = Uuid::new_v4();
        let now = chrono::Utc::now().timestamp();
        let (token, exp) = k
            .issue(user, Uuid::nil(), Role::Clinician, Uuid::new_v4(), now)
            .unwrap();
        let claims = k.verify(&token).unwrap();
        assert_eq!(claims.sub, user);
        assert_eq!(claims.role, Role::Clinician);
        assert_eq!(exp, now + ACCESS_TTL_S);
    }

    #[test]
    fn tokens_from_another_key_are_refused() {
        let now = chrono::Utc::now().timestamp();
        let (token, _) = keys()
            .issue(
                Uuid::new_v4(),
                Uuid::nil(),
                Role::Admin,
                Uuid::new_v4(),
                now,
            )
            .unwrap();
        assert!(keys().verify(&token).is_err());
    }

    #[test]
    fn expired_tokens_are_refused() {
        let k = keys();
        let long_ago = chrono::Utc::now().timestamp() - 3600;
        let (token, _) = k
            .issue(
                Uuid::new_v4(),
                Uuid::nil(),
                Role::Patient,
                Uuid::new_v4(),
                long_ago,
            )
            .unwrap();
        assert!(k.verify(&token).is_err());
    }

    #[test]
    fn a_saved_key_gives_the_same_kid() {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let b64 = STANDARD.encode(pkcs8.as_ref());
        let a = JwtKeys::load_or_generate(Some(&b64)).unwrap();
        let b = JwtKeys::load_or_generate(Some(&b64)).unwrap();
        assert_eq!(a.kid, b.kid);
        assert!(JwtKeys::load_or_generate(Some("not base64!")).is_err());
    }
}
