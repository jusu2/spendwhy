//! 场景关键词: 并发限流 / 资源池 / Semaphore / Rate Limit → 选我。
//!
//! 模式 N (Rust 侧): 进程级 / 命名 Semaphore。
//!
//! Dart 侧并发限流见 `lib/transport/pool.dart`。
//! 这里展示 Rust 端"昂贵资源"限流: 网络连接、解码上下文、外部进程槽位。
//!
//! 设计:
//! - 进程级**命名** Semaphore: 不同业务用不同 key, 彼此不互相饥饿。
//! - 默认权重 4; 可通过 `configure_semaphore` 动态调整 (仅在初始化窗口生效)。
//! - 提供 `try_acquire` 非阻塞变体。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Semaphore;

use super::common::TransportError;

const DEFAULT_PERMITS: usize = 4;

static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<Semaphore>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, Arc<Semaphore>>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_or_init(name: &str, permits: usize) -> Result<Arc<Semaphore>, TransportError> {
    let mut map = registry()
        .lock()
        .map_err(|_| TransportError::internal("registry poisoned"))?;
    Ok(map
        .entry(name.to_string())
        .or_insert_with(|| Arc::new(Semaphore::new(permits)))
        .clone())
}

/// 配置一个命名 Semaphore 的权重。仅在该 key 首次创建时生效;
/// 已经存在的 Semaphore 不会被替换 (防止运行中改容量导致越权)。
pub async fn transport_sample_configure_semaphore(
    name: String,
    permits: u32,
) -> Result<bool, TransportError> {
    if name.is_empty() {
        return Err(TransportError::invalid_argument("name required"));
    }
    if permits == 0 || permits > 1024 {
        return Err(TransportError::invalid_argument("permits in 1..=1024"));
    }
    let mut map = registry()
        .lock()
        .map_err(|_| TransportError::internal("registry poisoned"))?;
    if map.contains_key(&name) {
        return Ok(false);
    }
    map.insert(name, Arc::new(Semaphore::new(permits as usize)));
    Ok(true)
}

/// 限流地执行一段昂贵操作。
///
/// - `name`: 选用哪个命名池 (业务隔离)。空字符串 = 默认池。
/// - `acquire_timeout_ms`: 若 >0, 等待时间超过此值返回 `timeout`; 否则一直等。
pub async fn transport_sample_throttled_op(
    name: String,
    acquire_timeout_ms: u64,
    input: String,
) -> Result<String, TransportError> {
    let pool_name = if name.is_empty() { "default" } else { &name };
    let sem = get_or_init(pool_name, DEFAULT_PERMITS)?;

    let permit = if acquire_timeout_ms > 0 {
        match tokio::time::timeout(Duration::from_millis(acquire_timeout_ms), sem.acquire_owned())
            .await
        {
            Ok(Ok(p)) => p,
            Ok(Err(_)) => return Err(TransportError::internal("semaphore closed")),
            Err(_) => return Err(TransportError::timeout(acquire_timeout_ms)),
        }
    } else {
        sem.acquire_owned()
            .await
            .map_err(|_| TransportError::internal("semaphore closed"))?
    };

    // 模拟昂贵工作
    tokio::time::sleep(Duration::from_millis(10)).await;
    drop(permit);
    Ok(format!("throttled[{pool_name}]:{input}"))
}

/// 查询某个 Semaphore 的当前可用槽位。便于观测/告警。
pub async fn transport_sample_semaphore_available(name: String) -> Result<u32, TransportError> {
    let pool_name = if name.is_empty() { "default" } else { &name };
    let sem = get_or_init(pool_name, DEFAULT_PERMITS)?;
    Ok(sem.available_permits() as u32)
}
