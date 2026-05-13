//! 场景关键词: panic 安全 / catch_unwind / 防 FFI 崩溃 → 选我。
//!
//! 模式 T: panic-safety 包装。
//!
//! 为什么需要:
//! - Rust panic 穿越 FFI 边界是 **未定义行为** (UB)。
//! - flutter_rust_bridge 默认会在 `pub` 入口加 `catch_unwind`, 但若你在自己的
//!   服务层 / `spawn` 出去的任务里 panic, 那些没有走过 FRB 入口, 仍会爬到 dylib 之外。
//! - 经验法则: **任何 `tokio::spawn(...)` 内部代码必须自己 `catch_unwind` 包一层**。
//!
//! 注意:
//! - `AssertUnwindSafe` 是一个 promise: 你保证即使 panic 也不会留下半成品状态。
//!   如果可能留下不一致状态 (持锁中、写一半文件), 把 Mutex 改成 `parking_lot::Mutex`
//!   (其 poison 行为更宽松), 或显式回滚。
//! - 把 panic 转成 `TransportError::internal` 后, **记录原始 panic message** 以便排查;
//!   但**不要**回传 Dart, 避免泄露内部状态。

use std::panic::AssertUnwindSafe;

use futures::FutureExt;

use super::common::TransportError;

/// 同步 panic-safe 包装: 把 `f()` 的 panic 转成 `TransportError::internal`。
#[flutter_rust_bridge::frb(ignore)]
pub fn catch_panic_sync<T, F: FnOnce() -> T>(f: F) -> Result<T, TransportError> {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => Ok(v),
        Err(payload) => {
            let msg = panic_msg(&payload);
            // 真实业务: tracing::error!(panic = msg, "panic crossed FFI guard");
            Err(TransportError::internal(format!("panic: {msg}")))
        }
    }
}

/// 异步 panic-safe 包装: 同上, 但作用于 `Future`。
#[flutter_rust_bridge::frb(ignore)]
pub async fn catch_panic_async<T, F: std::future::Future<Output = T>>(
    fut: F,
) -> Result<T, TransportError> {
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(v) => Ok(v),
        Err(payload) => {
            let msg = panic_msg(&payload);
            Err(TransportError::internal(format!("panic: {msg}")))
        }
    }
}

fn panic_msg(payload: &(dyn std::any::Any + Send)) -> &str {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "<non-string panic payload>"
    }
}

/// 演示入口: Dart 调它会看到一个 `TransportError::internal("panic: ...")` 而非进程崩溃。
pub async fn transport_sample_panic_demo(should_panic: bool) -> Result<String, TransportError> {
    catch_panic_async(async move {
        if should_panic {
            panic!("simulated panic inside spawned async work");
        }
        "ok".to_string()
    })
    .await
}
