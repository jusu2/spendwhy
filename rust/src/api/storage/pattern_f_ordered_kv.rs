//! 场景关键词: 有序 KV / range scan / 前缀迭代 / 二级索引基础 / sled → 选我。
//!
//! 模式 F: 有序键值存储 (sled 包装)。
//!
//! 暴露 put / get / delete / scan_prefix / range 接口。键按字节字典序排列,
//! 适合做"按时间范围"或"按 user_id:" 前缀查询。
//!
//! 适用: 索引、有序集合、范围查询。
//! 不适用: 重启清空的临时缓存 (用模式 A); 高频 update 同 key (考虑批量写)。
//!
//! 注意:
//! - sled 0.34 是 lock-free, 多线程安全, 但单 DB 不能跨进程。
//! - 每个 `path` 内部用 `OnceLock` 维护一个 sled::Db 实例池, 同路径复用。
//! - `flush` 入口让上层在合适时点持久化; 默认 sled 后台异步 flush。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::common::StorageError;

#[derive(Debug, Clone)]
pub struct StorageSampleKvEntryDto {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

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

pub async fn storage_sample_kv_put(
    path: String,
    key: Vec<u8>,
    value: Vec<u8>,
) -> Result<(), StorageError> {
    if key.is_empty() {
        return Err(StorageError::invalid_argument("key must not be empty"));
    }
    let db = open_db(&path)?;
    tokio::task::spawn_blocking(move || db.insert(&key, value).map(|_| ()).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(())
}

pub async fn storage_sample_kv_get(
    path: String,
    key: Vec<u8>,
) -> Result<Option<Vec<u8>>, StorageError> {
    let db = open_db(&path)?;
    let v = tokio::task::spawn_blocking(move || db.get(&key).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(v.map(|iv| iv.to_vec()))
}

pub async fn storage_sample_kv_delete(
    path: String,
    key: Vec<u8>,
) -> Result<bool, StorageError> {
    let db = open_db(&path)?;
    let removed = tokio::task::spawn_blocking(move || db.remove(&key).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(removed.is_some())
}

/// 按前缀扫描; `limit=0` 表示不限。
pub async fn storage_sample_kv_scan_prefix(
    path: String,
    prefix: Vec<u8>,
    limit: u64,
) -> Result<Vec<StorageSampleKvEntryDto>, StorageError> {
    let db = open_db(&path)?;
    let out = tokio::task::spawn_blocking(move || -> Result<Vec<_>, StorageError> {
        let mut out = Vec::new();
        for r in db.scan_prefix(&prefix) {
            let (k, v) = r.map_err(map_sled_err)?;
            out.push(StorageSampleKvEntryDto {
                key: k.to_vec(),
                value: v.to_vec(),
            });
            if limit > 0 && out.len() as u64 >= limit {
                break;
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(out)
}

/// 范围查询 `[from, to)`; `limit=0` 表示不限。
pub async fn storage_sample_kv_range(
    path: String,
    from: Vec<u8>,
    to: Vec<u8>,
    limit: u64,
) -> Result<Vec<StorageSampleKvEntryDto>, StorageError> {
    let db = open_db(&path)?;
    let out = tokio::task::spawn_blocking(move || -> Result<Vec<_>, StorageError> {
        let mut out = Vec::new();
        for r in db.range(from..to) {
            let (k, v) = r.map_err(map_sled_err)?;
            out.push(StorageSampleKvEntryDto {
                key: k.to_vec(),
                value: v.to_vec(),
            });
            if limit > 0 && out.len() as u64 >= limit {
                break;
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(out)
}

pub async fn storage_sample_kv_flush(path: String) -> Result<u64, StorageError> {
    let db = open_db(&path)?;
    let n = tokio::task::spawn_blocking(move || db.flush().map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(n as u64)
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
