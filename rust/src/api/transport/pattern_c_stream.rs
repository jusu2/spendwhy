//! 场景关键词: 持续事件 / 订阅 / 实时推送 (Rust→Dart) → 选我。
//!
//! 模式 C: Stream 单向订阅 + 协作式背压。
//!
//! 何时用:
//! - Rust 产生一序列事件 (DB 变更、传感器、解码进度)。
//! - Dart 用 `listen` 持续消费。
//!
//! 关键约束:
//! - **可取消**: Dart 端 cancel subscription → Rust 端 `sink.add` 失败 → 循环退出。
//! - **背压**: hot loop 必须在每次 `add` 后 `tokio::task::yield_now()` 让出, 避免 sink 缓冲膨胀。
//! - **节流**: 高频生产时 (>1kHz), 在 Rust 端聚合后再 emit, 而非依赖 Dart 端 throttle。
//!
//! 何时不用:
//! - 还要从 Dart 反向送命令 → [`super::pattern_h_duplex`]。
//! - 全局多订阅者 → [`super::pattern_i_event_bus`]。

use std::time::Duration;

use crate::frb_generated::StreamSink;

use super::common::TransportError;

#[derive(Debug, Clone)]
pub struct TransportSampleTickDto {
    pub seq: u64,
    pub timestamp_ms: i64,
}

/// 每 `interval_ms` 毫秒推送一个 tick, 共 `count` 个。
///
/// `interval_ms = 0` 表示尽可能快地推送 (会让出调度避免饥饿)。
pub async fn transport_sample_ticks(
    sink: StreamSink<TransportSampleTickDto>,
    interval_ms: u64,
    count: u64,
) -> Result<(), TransportError> {
    if count > 1_000_000 {
        return Err(TransportError::invalid_argument(
            "count too large (max 1_000_000)",
        ));
    }
    let interval = Duration::from_millis(interval_ms);
    for seq in 0..count {
        let dto = TransportSampleTickDto {
            seq,
            timestamp_ms: now_ms(),
        };
        // 若 Dart 端已取消订阅, sink.add 返回 Err, 我们提前结束。
        if sink.add(dto).is_err() {
            return Ok(());
        }
        if interval.is_zero() {
            // 让出 executor, 避免 hot-loop 饥饿其它任务。
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(interval).await;
        }
    }
    Ok(())
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
