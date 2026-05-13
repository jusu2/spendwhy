//! 场景关键词: 幂等接收方 / idempotency key / 重试安全 → 选我。
//!
//! 模式 L (Rust 侧): 幂等接收方 + 有界 TTL 缓存。
//!
//! Dart 侧重试逻辑见 `lib/transport/retry.dart`。本文件演示 Rust 侧
//! 如何对相同 `idempotency_key` 直接返回缓存结果, 避免重复执行副作用。
//!
//! 实现要点:
//! - **有界**: `MAX_ENTRIES` 上限 + 简单 FIFO 淘汰; 真实系统应换成 `lru::LruCache`。
//! - **TTL**: 每条记录带过期时间戳; 读时若过期视为缺失。
//! - **线程安全**: `std::sync::Mutex` (持锁时间短, 不跨 await)。

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::common::TransportError;

/// 缓存最多保留多少条幂等记录。生产环境应根据流量调整或换 LRU。
const MAX_ENTRIES: usize = 1024;
/// 单条记录有效期。
const DEFAULT_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone)]
pub struct TransportSampleReceiptDto {
    pub key: String,
    pub result: String,
    pub deduped: bool,
}

struct CacheEntry {
    value: String,
    expires_at: Instant,
}

struct IdempotencyCache {
    map: HashMap<String, CacheEntry>,
    order: VecDeque<String>,
}

impl IdempotencyCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        let stale = match self.map.get(key) {
            Some(entry) if entry.expires_at <= Instant::now() => true,
            Some(entry) => return Some(entry.value.clone()),
            None => return None,
        };
        if stale {
            self.map.remove(key);
        }
        None
    }

    fn put(&mut self, key: String, value: String, ttl: Duration) {
        if self.map.contains_key(&key) {
            self.map.insert(
                key,
                CacheEntry {
                    value,
                    expires_at: Instant::now() + ttl,
                },
            );
            return;
        }
        while self.map.len() >= MAX_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
        self.order.push_back(key.clone());
        self.map.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }
}

static CACHE: OnceLock<Mutex<IdempotencyCache>> = OnceLock::new();

fn cache() -> &'static Mutex<IdempotencyCache> {
    CACHE.get_or_init(|| Mutex::new(IdempotencyCache::new()))
}

fn with_cache<R>(f: impl FnOnce(&mut IdempotencyCache) -> R) -> Result<R, TransportError> {
    let mut guard = cache()
        .lock()
        .map_err(|_| TransportError::internal("lock poisoned"))?;
    Ok(f(&mut guard))
}

/// 接收一条"具有副作用"的请求; 相同 `idempotency_key` 在 TTL 内只执行一次。
pub async fn transport_sample_apply_once(
    idempotency_key: String,
    payload: String,
) -> Result<TransportSampleReceiptDto, TransportError> {
    if idempotency_key.is_empty() {
        return Err(TransportError::invalid_argument("idempotency_key required"));
    }
    if idempotency_key.len() > 256 {
        return Err(TransportError::invalid_argument(
            "idempotency_key too long (max 256)",
        ));
    }

    let cached = with_cache(|c| c.get(&idempotency_key))?;
    if let Some(prior) = cached {
        return Ok(TransportSampleReceiptDto {
            key: idempotency_key,
            result: prior,
            deduped: true,
        });
    }

    let result = format!("applied:{payload}");
    with_cache(|c| c.put(idempotency_key.clone(), result.clone(), DEFAULT_TTL))?;
    Ok(TransportSampleReceiptDto {
        key: idempotency_key,
        result,
        deduped: false,
    })
}

/// 手动清理 (用于测试或维护)。返回清理前的条数。
pub async fn transport_sample_idempotency_cache_clear() -> Result<u32, TransportError> {
    with_cache(|c| {
        let n = c.map.len();
        c.map.clear();
        c.order.clear();
        n as u32
    })
}
