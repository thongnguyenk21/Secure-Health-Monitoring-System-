//! Password hashing with argon2id (REQ-SEC-012).

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

/// Shortest accepted password, characters (OpenAPI `minLength`).
pub const MIN_LEN: usize = 12;
/// Longest accepted password, bytes (OpenAPI `maxLength`).
pub const MAX_LEN: usize = 256;

/// Hashes a password into a PHC string.
///
/// Argon2id with the crate defaults (19 MiB memory, 2 passes). This takes tens
/// of milliseconds on purpose; call it from `spawn_blocking`.
///
/// # Errors
/// Returns the hashing error; it does not happen with the default parameters.
pub fn hash(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

/// Checks a password against a stored PHC string.
///
/// # Returns
/// `true` only for a match; a malformed stored hash is a mismatch.
#[must_use]
pub fn verify(password: &str, phc: &str) -> bool {
    PasswordHash::new(phc).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

/// A precomputed hash used when the email is unknown, so that a login for an
/// unknown account costs the same time as one for a known account.
///
/// # Errors
/// Returns the hashing error; it does not happen with the default parameters.
pub fn dummy_hash() -> Result<String, argon2::password_hash::Error> {
    hash("timing-equaliser-password")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_right_password_matches() {
        let phc = hash("correct horse battery").unwrap();
        assert!(phc.starts_with("$argon2id$"));
        assert!(verify("correct horse battery", &phc));
        assert!(!verify("wrong horse battery", &phc));
    }

    #[test]
    fn a_malformed_hash_never_matches() {
        assert!(!verify("anything", "not-a-phc-string"));
    }
}
