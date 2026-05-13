//! 场景关键词: 字段级加密 / PII / Token / AES-256-GCM / 信封 → 选我。
//!
//! 模式 I: 字段级信封加密 (AES-256-GCM)。
//!
//! 输入: 数据密钥 (32 字节, 调用方提供) + 明文 + AAD (associated data; 不加密但
//! 参与认证, 防替换攻击; 例如同记录的 user_id)。输出: 单个 `Vec<u8>` =
//! `[nonce: 12B][ciphertext + tag]`。
//!
//! 关键设计:
//! - **数据密钥由调用方提供**: 库不保管密钥。生产用法是 Dart 端从 SecureStorage
//!   (模式 K) 取主密钥, 解出数据密钥后传进来 — 这就是 envelope 模式。
//! - **nonce 每次随机生成**: AES-GCM 重用 nonce + 同 key 会泄漏密钥, 务必新鲜随机。
//! - **AAD 是契约**: 加密时传什么, 解密时必须传同样的; 解密时传错 → tag 校验失败 →
//!   `corrupted` 错误。
//!
//! 不适用: 全表加密 (用 SQLCipher); 不可信对方持有密钥的场景 (用非对称加密)。

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::{Aes256Gcm, Key, Nonce};

use super::common::StorageError;

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
/// 单条加密载荷上限: 16MB (AES-GCM 单 nonce 安全上限是 64GB, 16MB 远远低于)。
const MAX_PLAINTEXT_LEN: usize = 16 * 1024 * 1024;

/// 加密: 返回 `[nonce(12) || ciphertext_with_tag]`。
pub fn storage_sample_envelope_encrypt(
    key: Vec<u8>,
    plaintext: Vec<u8>,
    aad: Vec<u8>,
) -> Result<Vec<u8>, StorageError> {
    if key.len() != KEY_LEN {
        return Err(StorageError::invalid_argument(format!(
            "key must be {KEY_LEN} bytes (got {})",
            key.len()
        )));
    }
    if plaintext.len() > MAX_PLAINTEXT_LEN {
        return Err(StorageError::quota_exceeded(format!(
            "plaintext > {MAX_PLAINTEXT_LEN}"
        )));
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| StorageError::internal("encrypt failed"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// 解密; 数据被改 / key 错 / AAD 错 → `corrupted`。
pub fn storage_sample_envelope_decrypt(
    key: Vec<u8>,
    envelope: Vec<u8>,
    aad: Vec<u8>,
) -> Result<Vec<u8>, StorageError> {
    if key.len() != KEY_LEN {
        return Err(StorageError::invalid_argument(format!(
            "key must be {KEY_LEN} bytes"
        )));
    }
    if envelope.len() < NONCE_LEN + 16 {
        return Err(StorageError::corrupted(
            "envelope too short (need nonce + tag)",
        ));
    }
    let (nonce_bytes, ciphertext) = envelope.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| StorageError::corrupted("decrypt failed (tag/aad/key mismatch)"))?;
    Ok(plaintext)
}

/// 工具: 生成新的 32 字节随机数据密钥 (CSPRNG)。
pub fn storage_sample_generate_data_key() -> Vec<u8> {
    let mut k = vec![0u8; KEY_LEN];
    OsRng.fill_bytes(&mut k);
    k
}
