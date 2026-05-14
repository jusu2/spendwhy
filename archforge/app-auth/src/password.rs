//! Argon2id 密码哈希与验证。
//!
//! - **算法**：Argon2id，OWASP 推荐的密码 KDF。
//! - **参数**：`m=19 MiB, t=2, p=1`（OWASP 2024 最低值）。
//!   "test_fast" 预设降为 `m=8 KiB, t=1, p=1`，避免单元测试
//!   每次调用耗费数秒。
//! - **盐**：每个哈希 16 随机字节，由 [`rand::rngs::OsRng`] 生成。
//! - **输出**：PHC 字符串（`$argon2id$v=19$…`）—— 自描述，
//!   验证方无需带外的参数信息。
//! - **验证路径** 相对存储的哈希为常数时间。
//!
//! Dummy 哈希（用于使 `verify_password` 在用户不存在时
//! 仍保持常数时间）在构造时计算一次并复用。

use archforge_contract_auth::{PasswordHash, PlainPassword};
use archforge_kernel::{AppError, Result};
use argon2::{Algorithm, Argon2, Params, Version as A2Version};
use password_hash::{
    rand_core::OsRng, PasswordHash as PhcHash, PasswordHasher as PhcHasher,
    PasswordVerifier as PhcVerifier, SaltString,
};
use secrecy::ExposeSecret;

/// 无状态的 argon2id 哈希器。
#[derive(Clone)]
pub struct PasswordHasher {
    argon: Argon2<'static>,
    dummy: PasswordHash,
}

impl PasswordHasher {
    /// OWASP 推荐的生产预设。
    pub fn production() -> Self {
        // m=19456 KiB, t=2, p=1, output=32 字节。
        let params = Params::new(19_456, 2, 1, Some(32))
            .expect("argon2 params: production preset must be valid");
        Self::with_params(params)
    }

    /// 单元测试用的快速预设（仍是真实的 argon2id，只是内存极小）。
    /// **切勿用于生产。**
    pub fn test_fast() -> Self {
        let params =
            Params::new(8, 1, 1, Some(32)).expect("argon2 params: test_fast preset must be valid");
        Self::with_params(params)
    }

    fn with_params(params: Params) -> Self {
        let argon = Argon2::new(Algorithm::Argon2id, A2Version::V0x13, params);
        // 预先计算一个 dummy 哈希，使 `verify_password` 在用户
        // 不存在时仍能进行真实的 argon2 验证。我们用一个
        // 固定字符串配上新鲜的盐 —— 它永远不会等于任何用户的密码。
        let salt = SaltString::generate(&mut OsRng);
        let dummy_phc = argon
            .hash_password(b"archforge-dummy-never-matches", &salt)
            .expect("dummy hash must succeed")
            .to_string();
        let dummy = PasswordHash::from_phc(dummy_phc).expect("dummy is valid PHC");
        Self { argon, dummy }
    }

    /// 预计算的 dummy，用以保持认证失败的计时常数。
    pub fn dummy_hash(&self) -> PasswordHash {
        self.dummy.clone()
    }

    /// 对明文密码进行哈希。
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

    /// 对照已存储的 PHC 哈希验证明文密码。不匹配时
    /// 返回 `false`（**非** 错误）；仅当存储的哈希
    /// 损坏时才返回 `Internal`。
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
        // 盐为随机，因此 dummy PHC 字符串不同。
        assert_ne!(a.dummy_hash().as_phc(), b.dummy_hash().as_phc());
    }

    #[test]
    fn malformed_stored_hash_returns_false_not_panic() {
        let h = PasswordHasher::test_fast();
        // 构造为：能通过 `from_phc` 的前缀检查，但 PHC 解析会失败。
        let bad = PasswordHash::from_phc("$argon2id$nonsense").unwrap();
        assert!(!h.verify(&PlainPassword::new("anything"), &bad));
    }
}
