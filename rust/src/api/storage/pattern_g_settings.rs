//! 场景关键词: 设置 / preferences / 小配置 / 明文 / 常读少写 → 选我。
//!
//! 模式 G: Settings KV。
//!
//! 与模式 F 共享 sled 后端, 但**强约束**:
//! - value ≤ 4096 字节 (防误把大数据塞进 settings)。
//! - 总 key 数 ≤ 4096 (防 unbounded growth)。
//! - 仅支持 string value (内部还是字节, 但接口强制 UTF-8)。
//!
//! 适用: 用户偏好 (theme, locale, lastTab) — 启动时一次性读完。
//! 不适用: 大对象 (用 C); 高写入 KV (用 F); 敏感数据 (用 I 加密或 Dart K)。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::common::StorageError;

const VALUE_MAX_BYTES: usize = 4096;
const MAX_KEYS: usize = 4096;
const KEY_MAX_LEN: usize = 128;

fn pool() -> &'static Mutex<HashMap<String, sled::Db>> {
    static POOL: OnceLock<Mutex<HashMap<String, sled::Db>>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn open_db(path: &str) -> Result<sled::Db, StorageError> {
    if path.is_empty() {
        return Err(StorageError::invalid_argument("path must not be empty"));
    }
    let mut p = pool()
        .lock()
        .map_err(|_| StorageError::internal("pool lock poisoned"))?;
    if let Some(db) = p.get(path) {
        return Ok(db.clone());
    }
    let db = sled::open(path).map_err(map_sled_err)?;
    p.insert(path.to_string(), db.clone());
    Ok(db)
}

pub async fn storage_sample_settings_set(
    path: String,
    key: String,
    value: String,
) -> Result<(), StorageError> {
    validate_key(&key)?;
    if value.len() > VALUE_MAX_BYTES {
        return Err(StorageError::quota_exceeded(format!(
            "value > {VALUE_MAX_BYTES} bytes"
        )));
    }
    let db = open_db(&path)?;
    tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
        // 检查与写入在同一阻塞线程内完成, 不再分别走 tokio 调度;
        // 注意 sled 没有跨 key 事务计数, 极并发下 MAX_KEYS 仍是软上限 (可能短暂超出 1~N)。
        let already = db.contains_key(key.as_bytes()).map_err(map_sled_err)?;
        if !already && db.len() >= MAX_KEYS {
            return Err(StorageError::quota_exceeded(format!(
                "settings reached {MAX_KEYS} keys"
            )));
        }
        db.insert(key.as_bytes(), value.as_bytes())
            .map(|_| ())
            .map_err(map_sled_err)
    })
    .await
    .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(())
}

pub async fn storage_sample_settings_get(
    path: String,
    key: String,
) -> Result<Option<String>, StorageError> {
    validate_key(&key)?;
    let db = open_db(&path)?;
    let v = tokio::task::spawn_blocking(move || db.get(key.as_bytes()).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    match v {
        Some(iv) => {
            let s = String::from_utf8(iv.to_vec())
                .map_err(|_| StorageError::corrupted("settings value not utf-8"))?;
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

pub async fn storage_sample_settings_delete(
    path: String,
    key: String,
) -> Result<bool, StorageError> {
    validate_key(&key)?;
    let db = open_db(&path)?;
    let v = tokio::task::spawn_blocking(move || db.remove(key.as_bytes()).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(v.is_some())
}

/// 一次读完所有 settings (启动期典型用法)。
pub async fn storage_sample_settings_dump(
    path: String,
) -> Result<Vec<(String, String)>, StorageError> {
    let db = open_db(&path)?;
    let out = tokio::task::spawn_blocking(move || -> Result<Vec<_>, StorageError> {
        let mut out = Vec::new();
        for r in db.iter() {
            let (k, v) = r.map_err(map_sled_err)?;
            let ks = String::from_utf8(k.to_vec())
                .map_err(|_| StorageError::corrupted("settings key not utf-8"))?;
            let vs = String::from_utf8(v.to_vec())
                .map_err(|_| StorageError::corrupted("settings value not utf-8"))?;
            out.push((ks, vs));
        }
        Ok(out)
    })
    .await
    .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(out)
}

fn validate_key(key: &str) -> Result<(), StorageError> {
    if key.is_empty() {
        return Err(StorageError::invalid_argument("key must not be empty"));
    }
    if key.len() > KEY_MAX_LEN {
        return Err(StorageError::invalid_argument(format!(
            "key > {KEY_MAX_LEN} chars"
        )));
    }
    if !key
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
    {
        return Err(StorageError::invalid_argument(
            "key may contain only [A-Za-z0-9._-]",
        ));
    }
    Ok(())
}

fn map_sled_err(e: sled::Error) -> StorageError {
    use sled::Error::*;
    match e {
        CollectionNotFound(_) => StorageError::not_found(e.to_string()),
        Unsupported(_) => StorageError::invalid_argument(e.to_string()),
        Corruption { .. } => StorageError::corrupted(e.to_string()),
        Io(io) => io.into(),
        ReportableBug(_) => StorageError::internal(e.to_string()),
    }
}
