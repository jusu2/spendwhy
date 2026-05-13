//! Argon2id password hashing & verification.
//!
//! - **Algorithm**: Argon2id, the OWASP-recommended password KDF.
//! - **Parameters**: `m=19 MiB, t=2, p=1` (OWASP minimum, 2024). The
//!   "test_fast" preset drops to `m=8 KiB, t=1, p=1` so unit tests don't
//!   take seconds per call.
//! - **Salt**: 16 random bytes per hash, generated via [`rand::rngs::OsRng`].
//! - **Output**: PHC string (`$argon2id$v=19$…`) — self-describing, so the
//!   verifier needs no out-of-band parameter knowledge.
//! - **Verification path** is constant time wrt the stored hash.
//!
//! The dummy hash (used to keep `verify_password` constant-time when the
//! user does not exist) is computed once at construction and reused.

use argon2::{Algorithm, Argon2, Params, Version as A2Version};
use archforge_contract_auth::{PasswordHash, PlainPassword};
use archforge_kernel::{AppError, Result};
use password_hash::{
    rand_core::OsRng, PasswordHash as PhcHash, PasswordHasher as PhcHasher,
    PasswordVerifier as PhcVerifier, SaltString,
};
use secrecy::ExposeSecret;

/// Stateless argon2id hasher.
#[derive(Clone)]
pub struct PasswordHasher {
    argon: Argon2<'static>,
    dummy: PasswordHash,
}

impl PasswordHasher {
    /// OWASP-recommended production preset.
    pub fn production() -> Self {
        // m=19456 KiB, t=2, p=1, output=32 bytes.
        let params = Params::new(19_456, 2, 1, Some(32))
            .expect("argon2 params: production preset must be valid");
        Self::with_params(params)
    }

    /// Fast preset for unit tests (still real argon2id, just tiny memory).
    /// **Never use in production.**
    pub fn test_fast() -> Self {
        let params = Params::new(8, 1, 1, Some(32))
            .expect("argon2 params: test_fast preset must be valid");
        Self::with_params(params)
    }

    fn with_params(params: Params) -> Self {
        let argon = Argon2::new(Algorithm::Argon2id, A2Version::V0x13, params);
        // Pre-compute a dummy hash so `verify_password` can run a real
        // argon2 verification when the user is missing. We hash a fixed
        // string with a fresh salt — it is never equal to a user's password.
        let salt = SaltString::generate(&mut OsRng);
        let dummy_phc = argon
            .hash_password(b"archforge-dummy-never-matches", &salt)
            .expect("dummy hash must succeed")
            .to_string();
        let dummy = PasswordHash::from_phc(dummy_phc).expect("dummy is valid PHC");
        Self { argon, dummy }
    }

    /// Pre-computed dummy used to keep auth-failure timing constant.
    pub fn dummy_hash(&self) -> PasswordHash {
        self.dummy.clone()
    }

    /// Hash a plain-text password.
    pub fn hash(&self, password: &PlainPassword) -> Result<PasswordHash> {
        let salt = SaltString::generate(&mut OsRng);
        let plain = password.0.expose_secret().as_bytes();
        let phc = self
            .argon
            .hash_password(plain, &salt)
            .map_err(|e| AppError::Internal(format!("argon2 hash: {e}")))?
            .to_string();
        PasswordHash::from_phc(phc)
    }

    /// Verify a plain-text password against a stored PHC hash. Returns
    /// `false` (NOT an error) on mismatch; `Internal` only on a malformed
    /// stored hash.
    pub fn verify(&self, password: &PlainPassword, stored: &PasswordHash) -> bool {
        let parsed = match PhcHash::new(stored.as_phc()) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let plain = password.0.expose_secret().as_bytes();
        self.argon.verify_password(plain, &parsed).is_ok()
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::production()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let h = PasswordHasher::test_fast();
        let pw = PlainPassword::new("correct horse battery staple");
        let stored = h.hash(&pw).unwrap();
        assert!(h.verify(&pw, &stored));
    }

    #[test]
    fn wrong_password_does_not_verify() {
        let h = PasswordHasher::test_fast();
        let stored = h.hash(&PlainPassword::new("right")).unwrap();
        assert!(!h.verify(&PlainPassword::new("wrong"), &stored));
    }

    #[test]
    fn dummy_hash_is_independent_per_construction() {
        let a = PasswordHasher::test_fast();
        let b = PasswordHasher::test_fast();
        // Salts are random, so dummy PHC strings differ.
        assert_ne!(a.dummy_hash().as_phc(), b.dummy_hash().as_phc());
    }

    #[test]
    fn malformed_stored_hash_returns_false_not_panic() {
        let h = PasswordHasher::test_fast();
        // Crafted to pass `from_phc`'s prefix check but fail PHC parsing.
        let bad = PasswordHash::from_phc("$argon2id$nonsense").unwrap();
        assert!(!h.verify(&PlainPassword::new("anything"), &bad));
    }
}
