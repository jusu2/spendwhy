//! 场景关键词: 用户可中途取消 / 长任务 / 长 IO → 选我。
//!
//! 模式 D: 异步 + `CancelHandle` (协作式取消)。
//!
//! Dart 侧:
//! ```dart
//! final cancel = await newCancelHandle();
//! final fut = transportSampleSlowJob(input: 'x', cancel: cancel);
//! Future.delayed(Duration(seconds: 1), () => cancel.cancel());
//! try { await fut; } on TransportError catch (e) { /* e.isCanceled */ }
//! ```
//!
//! 实现要点:
//! - **协作式**: Rust 端必须在循环中 `token.check()` 或 `select!` `wait_cancelled()`。
//! - **结构化清理**: 在 `?` 早退之前完成必要的释放; 这里用 RAII (drop)。
//! - **不要复用句柄**: 一个 `CancelHandle` 对应一个任务; 否则一次 cancel 会取消多任务。

use std::time::{Duration, Instant};

use super::common::{CancelHandle, TransportError};

/// 分阶段长任务: 每步检查取消位; 取消时返回 `canceled` 且包含已完成步数。
pub async fn transport_sample_slow_job(
    input: String,
    steps: u32,
    step_ms: u64,
    cancel: &CancelHandle,
) -> Result<String, TransportError> {
    if steps == 0 || steps > 10_000 {
        return Err(TransportError::invalid_argument("steps must be in 1..=10000"));
    }
    let token = cancel.token();
    let started = Instant::now();
    let step_duration = Duration::from_millis(step_ms.max(1));
    let mut completed = 0u32;
    for _ in 0..steps {
        token.check()?;
        tokio::time::sleep(step_duration).await;
        completed += 1;
    }
    token.check()?;

    Ok(format!(
        "done:{input}:{completed}/{steps}steps:{}ms",
        started.elapsed().as_millis()
    ))
}
