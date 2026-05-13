//! 场景关键词: 依赖反转 / Dart 注入回调 / Rust 调用 Dart 函数 → 选我。
//!
//! 模式 M: `DartFn` 闭包。
//!
//! 典型场景: Rust 业务需要某种 IO 能力 (密钥提供、网络拉取、用户确认),
//! 但具体实现在 Dart 侧 (`flutter_secure_storage`、HTTP client 等)。
//! 解法: Rust 入口接受一个 `impl Fn(...) -> DartFnFuture<...>` 参数。
//!
//! 注意:
//! - `DartFn*` 由 FRB 生成绑定; Rust 端调用时必须在异步上下文。
//! - 不要在持锁状态下 `await` Dart 回调 (重入死锁)。
//! - 必须给回调加超时 (Dart 端可能崩溃 / 永不返回)。
//! - 回调抛异常时 FRB 会把它打包成 `anyhow::Error`; 这里翻译为 `TransportError`。

use std::time::Duration;

use flutter_rust_bridge::DartFnFuture;

use super::common::TransportError;

/// 让 Dart 提供一个键值查询函数; Rust 用它解决"密钥/配置"问题。
///
/// `per_key_timeout_ms` 防止单个回调阻塞整个批处理。
pub async fn transport_sample_with_kv_provider(
    keys: Vec<String>,
    per_key_timeout_ms: u64,
    get: impl Fn(String) -> DartFnFuture<Option<String>>,
) -> Result<Vec<Option<String>>, TransportError> {
    if keys.len() > 1024 {
        return Err(TransportError::invalid_argument("too many keys (max 1024)"));
    }
    let timeout = Duration::from_millis(per_key_timeout_ms.max(1));
    let mut out = Vec::with_capacity(keys.len());
    for k in keys {
        let key_for_err = k.clone();
        let fut = get(k);
        match tokio::time::timeout(timeout, fut).await {
            Ok(v) => out.push(v),
            Err(_) => {
                return Err(TransportError::timeout(per_key_timeout_ms))
                    .map_err(|mut e: TransportError| {
                        e.message = format!("kv provider timed out on key '{key_for_err}'");
                        e
                    });
            }
        }
    }
    Ok(out)
}

/// 单次确认 (yes/no) 弹窗回调示例。Dart 侧实现 UI 对话框, Rust 业务等待结果。
pub async fn transport_sample_with_confirm(
    prompt: String,
    timeout_ms: u64,
    confirm: impl Fn(String) -> DartFnFuture<bool>,
) -> Result<bool, TransportError> {
    if prompt.is_empty() {
        return Err(TransportError::invalid_argument("empty prompt"));
    }
    let timeout = Duration::from_millis(timeout_ms.max(1));
    match tokio::time::timeout(timeout, confirm(prompt)).await {
        Ok(yes) => Ok(yes),
        Err(_) => Err(TransportError::timeout(timeout_ms)),
    }
}
