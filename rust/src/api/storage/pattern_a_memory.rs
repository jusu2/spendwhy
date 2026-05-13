//! 场景关键词: 内存缓存 / TTL / LRU / 进程内不持久 / 命中率 → 选我。
//!
//! 模式 A: 进程内 TTL + LRU 缓存。
//!
//! 用 `RustOpaque<StorageSampleMemoryCache>` 句柄, 每个缓存实例独立配置 capacity
//! 和默认 TTL。所有方法同步, 内部用 `std::sync::Mutex` 串行化, 无锁
//! 升级路径 (sample 只示范一种合理实现; 高并发场景可换 dashmap / parking_lot)。
//!
//! 适用: 短期反向索引、computed-once、热点查询去重。
//! 不适用: 重启后还要的 (用模式 H)、大对象 (用模式 C)。
//!
//! 设计要点:
//! - eviction = `capacity` 限制 (FIFO 顺序近似 LRU: 写入或读取都重新插队)。
//! - TTL 在**读时**惰性校验; 不开后台清理线程, 避免引入 tokio::time 复杂度。
//! - `gc_expired()` 显式入口让上层在合适时机调用 (例如低活动期)。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use super::common::StorageError;

const DEFAULT_CAPACITY: usize = 1024;

struct Entry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
    /// 用于 LRU 排序的单调序号; 越大越新。
    seq: u64,
}

struct Inner {
    map: HashMap<String, Entry>,
    capacity: usize,
    default_ttl_ms: Option<u64>,
    next_seq: u64,
    hits: u64,
    misses: u64,
}

pub struct StorageSampleMemoryCache {
    inner: Mutex<Inner>,
}

#[derive(Debug, Clone)]
pub struct StorageSampleCacheStatsDto {
    pub len: u64,
    pub capacity: u64,
    pub hits: u64,
    pub misses: u64,
}

impl StorageSampleMemoryCache {
    /// 工厂: `capacity=0` 时使用默认值 1024。
    pub fn open(capacity: u64, default_ttl_ms: Option<u64>) -> Self {
        let cap = if capacity == 0 {
            DEFAULT_CAPACITY
        } else {
            capacity as usize
        };
        Self {
            inner: Mutex::new(Inner {
                map: HashMap::with_capacity(cap.min(1024)),
                capacity: cap,
                default_ttl_ms,
                next_seq: 0,
                hits: 0,
                misses: 0,
            }),
        }
    }

    /// 写入; 若超出 capacity, 淘汰最旧条目。`ttl_ms=None` 用默认 TTL。
    pub fn put(
        &self,
        key: String,
        value: Vec<u8>,
        ttl_ms: Option<u64>,
    ) -> Result<(), StorageError> {
        if key.is_empty() {
            return Err(StorageError::invalid_argument("key must not be empty"));
        }
        let mut g = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("lock poisoned"))?;
        let ttl = ttl_ms.or(g.default_ttl_ms);
        let expires_at = ttl.map(|ms| Instant::now() + std::time::Duration::from_millis(ms));
        g.next_seq += 1;
        let seq = g.next_seq;
        g.map.insert(key, Entry { value, expires_at, seq });
        evict_if_needed(&mut g);
        Ok(())
    }

    /// 读取; 命中返回 `Some(value)` 并更新 LRU 序号。已过期视为 miss 并删除。
    pub fn get(&self, key: String) -> Result<Option<Vec<u8>>, StorageError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("lock poisoned"))?;
        let now = Instant::now();
        let expired = match g.map.get(&key) {
            Some(e) => e.expires_at.is_some_and(|t| t <= now),
            None => {
                g.misses += 1;
                return Ok(None);
            }
        };
        if expired {
            g.map.remove(&key);
            g.misses += 1;
            return Ok(None);
        }
        g.next_seq += 1;
        let seq = g.next_seq;
        let value = {
            let entry = g.map.get_mut(&key).expect("just checked");
            entry.seq = seq;
            entry.value.clone()
        };
        g.hits += 1;
        Ok(Some(value))
    }

    pub fn delete(&self, key: String) -> Result<bool, StorageError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("lock poisoned"))?;
        Ok(g.map.remove(&key).is_some())
    }

    /// 删除所有已过期条目, 返回删除数量。
    pub fn gc_expired(&self) -> Result<u64, StorageError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("lock poisoned"))?;
        let now = Instant::now();
        let before = g.map.len();
        g.map
            .retain(|_, e| !e.expires_at.is_some_and(|t| t <= now));
        Ok((before - g.map.len()) as u64)
    }

    pub fn stats(&self) -> Result<StorageSampleCacheStatsDto, StorageError> {
        let g = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("lock poisoned"))?;
        Ok(StorageSampleCacheStatsDto {
            len: g.map.len() as u64,
            capacity: g.capacity as u64,
            hits: g.hits,
            misses: g.misses,
        })
    }

    pub fn clear(&self) -> Result<(), StorageError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| StorageError::internal("lock poisoned"))?;
        g.map.clear();
        Ok(())
    }
}

fn evict_if_needed(g: &mut Inner) {
    while g.map.len() > g.capacity {
        let oldest = g
            .map
            .iter()
            .min_by_key(|(_, e)| e.seq)
            .map(|(k, _)| k.clone());
        if let Some(k) = oldest {
            g.map.remove(&k);
        } else {
            break;
        }
    }
}

/// 工厂入口: Dart 侧 `await storageSampleOpenMemoryCache(...)`。
pub fn storage_sample_open_memory_cache(
    capacity: u64,
    default_ttl_ms: Option<u64>,
) -> StorageSampleMemoryCache {
    StorageSampleMemoryCache::open(capacity, default_ttl_ms)
}
