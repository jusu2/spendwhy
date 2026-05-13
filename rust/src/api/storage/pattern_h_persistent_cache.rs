//! 场景关键词: 持久缓存 / 重启保留 / TTL / sled + 过期清理 → 选我。
//!
//! 模式 H: 持久 TTL 缓存 (sled 后端)。
//!
//! 与模式 A 同语义但跨进程持久。每个 value 头部嵌 16 字节 header:
//! `[expires_at_ms: u64_be][_reserved: 8 bytes]`。读时检查过期 → 惰性删除。
//! `gc_expired` 入口让上层在低活动期手动清扫。
//!
//! 适用: HTTP 响应缓存、computed-expensive 结果 (跨 app 启动有效)。
//! 不适用: 不能容忍延迟过期的强一致场景。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::common::StorageError;

const HEADER_LEN: usize = 16;

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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn pack(expires_at_ms: u64, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.extend_from_slice(&expires_at_ms.to_be_bytes());
    out.extend_from_slice(&[0u8; 8]); // reserved
    out.extend_from_slice(body);
    out
}

fn unpack(raw: &[u8]) -> Result<(u64, &[u8]), StorageError> {
    if raw.len() < HEADER_LEN {
        return Err(StorageError::corrupted("cache entry shorter than header"));
    }
    let mut be = [0u8; 8];
    be.copy_from_slice(&raw[..8]);
    let exp = u64::from_be_bytes(be);
    Ok((exp, &raw[HEADER_LEN..]))
}

pub async fn storage_sample_pcache_put(
    path: String,
    key: Vec<u8>,
    value: Vec<u8>,
    ttl_ms: u64,
) -> Result<(), StorageError> {
    if key.is_empty() {
        return Err(StorageError::invalid_argument("key must not be empty"));
    }
    if ttl_ms == 0 {
        return Err(StorageError::invalid_argument("ttl_ms must be > 0"));
    }
    let db = open_db(&path)?;
    let exp = now_ms().saturating_add(ttl_ms);
    let packed = pack(exp, &value);
    tokio::task::spawn_blocking(move || db.insert(&key, packed).map(|_| ()).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(())
}

pub async fn storage_sample_pcache_get(
    path: String,
    key: Vec<u8>,
) -> Result<Option<Vec<u8>>, StorageError> {
    let db = open_db(&path)?;
    let key_clone = key.clone();
    let raw = tokio::task::spawn_blocking(move || db.get(&key_clone).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    let raw = match raw {
        Some(iv) => iv,
        None => return Ok(None),
    };
    let (exp, body) = unpack(&raw)?;
    if exp <= now_ms() {
        let db2 = open_db(&path)?;
        let k2 = key;
        let _ = tokio::task::spawn_blocking(move || db2.remove(&k2)).await;
        return Ok(None);
    }
    Ok(Some(body.to_vec()))
}

pub async fn storage_sample_pcache_delete(
    path: String,
    key: Vec<u8>,
) -> Result<bool, StorageError> {
    let db = open_db(&path)?;
    let removed = tokio::task::spawn_blocking(move || db.remove(&key).map_err(map_sled_err))
        .await
        .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(removed.is_some())
}

/// 遍历清除所有已过期条目。
pub async fn storage_sample_pcache_gc_expired(path: String) -> Result<u64, StorageError> {
    let db = open_db(&path)?;
    let removed = tokio::task::spawn_blocking(move || -> Result<u64, StorageError> {
        let now = now_ms();
        let mut dropped = 0u64;
        let mut to_drop = Vec::new();
        for r in db.iter() {
            let (k, v) = r.map_err(map_sled_err)?;
            let (exp, _) = unpack(&v)?;
            if exp <= now {
                to_drop.push(k.to_vec());
            }
        }
        for k in to_drop {
            if db.remove(&k).map_err(map_sled_err)?.is_some() {
                dropped += 1;
            }
        }
        Ok(dropped)
    })
    .await
    .map_err(|e| StorageError::internal(e.to_string()))??;
    Ok(removed)
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
