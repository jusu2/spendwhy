//! 场景关键词: 结构化并发 / fan-out / parallel / select / 多任务协调取消 → 选我。
//!
//! 模式 U: 结构化并发 (fan-out + fan-in + coordinated cancel)。
//!
//! 何时用:
//! - 同时调用 N 个子任务, 任一失败立刻让其他子任务也停。
//! - "尽快返回第一个成功" (`select!` ok-first) 或 "全部完成" (`join_all`)。
//!
//! 关键约束:
//! - **不要 `tokio::spawn` 然后忘记 join**: 那是无结构并发, 任务会脱缰。
//! - **取消语义**: drop 一个 `JoinHandle` 不会取消任务; 用 `CancelHandle` 配合。
//! - **错误传播**: 任一子任务失败时, 用 `select! { biased; }` 或显式 `cancel.cancel()`。

use std::time::Duration;

use super::common::{CancelHandle, TransportError};

#[derive(Debug, Clone)]
pub struct TransportSampleFanoutResult {
    pub completed: u32,
    pub partial: Vec<String>,
}

/// fan-out: 并行执行 N 个子任务, 任一失败则取消其余, 返回所有成功结果 + 第一个失败。
///
/// `inputs` 限 32 个以内, 防止意外提交大批任务。
pub async fn transport_sample_fanout(
    inputs: Vec<String>,
    per_task_ms: u64,
    cancel: &CancelHandle,
) -> Result<TransportSampleFanoutResult, TransportError> {
    if inputs.is_empty() {
        return Err(TransportError::invalid_argument("inputs required"));
    }
    if inputs.len() > 32 {
        return Err(TransportError::invalid_argument("too many inputs (max 32)"));
    }

    let token = cancel.token();

    // 用 futures::future::join_all 而非 tokio::spawn: 任务在当前 future 树上,
    // 调用方 drop 整个 future 即可级联取消所有子任务。
    let tasks = inputs.into_iter().map(|input| {
        let token = token.clone();
        async move {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(per_task_ms)) => {
                    if token.is_cancelled() {
                        Err(TransportError::canceled())
                    } else {
                        Ok(format!("done:{input}"))
                    }
                }
                _ = wait_cancel(token.clone()) => Err(TransportError::canceled()),
            }
        }
    });

    let results: Vec<Result<String, TransportError>> = futures::future::join_all(tasks).await;

    // fan-in: 收集结果。第一个错误立刻返回 (此时其它任务已被 cancel 或 sleep 完毕)。
    let mut partial = Vec::with_capacity(results.len());
    let mut first_err: Option<TransportError> = None;
    for r in results {
        match r {
            Ok(v) => partial.push(v),
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
                // 让其余 inflight 任务有机会观察到 cancel
                cancel.cancel();
            }
        }
    }

    if let Some(e) = first_err {
        return Err(e);
    }
    let completed = partial.len() as u32;
    Ok(TransportSampleFanoutResult { completed, partial })
}

/// 内部辅助: 转 `is_cancelled()` 轮询为 Future。
async fn wait_cancel(token: super::common::CancelToken) {
    loop {
        if token.is_cancelled() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// 取第一个成功者; 其余子任务 (在 select! 中) 因 drop 而自然停止。
pub async fn transport_sample_race(
    candidates: Vec<String>,
    per_task_ms: u64,
) -> Result<String, TransportError> {
    if candidates.is_empty() {
        return Err(TransportError::invalid_argument("candidates required"));
    }
    let tasks: Vec<_> = candidates
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let delay = Duration::from_millis(per_task_ms + (i as u64) * 5);
            async move {
                tokio::time::sleep(delay).await;
                c
            }
        })
        .collect();

    let (winner, _index, _rest) = futures::future::select_all(tasks.into_iter().map(Box::pin)).await;
    Ok(winner)
}
