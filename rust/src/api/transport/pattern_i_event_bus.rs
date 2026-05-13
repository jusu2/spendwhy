//! 场景关键词: 全局事件总线 / 多订阅者 / Rust 主动通知 UI / 快照 → 选我。
//!
//! 模式 I: 进程级 broadcast + 最近事件快照。
//!
//! 用 `tokio::sync::broadcast::Sender` 作为单例。任意 Rust 模块 `publish_event()`,
//! 多个 Dart 订阅者 `subscribe_events()` 同时收。
//!
//! 设计:
//! - **历史快照**: 新订阅者首先收到最近 N 条事件 (按 topic 去重保留最新), 然后衔接实时流。
//! - **lagged 上报**: 慢消费者跳过事件时, 推送一条 `__lagged__` 系统事件而非静默丢弃。
//! - **`include_snapshot` 开关**: 调用方可选择是否要历史。
//!
//! 与模式 C 的差异:
//! - C 是一次性 `fn -> Stream`, 流和函数生命周期绑死。
//! - I 是常驻总线, 生命周期跨整个进程, 可有 0..N 订阅者。
//!
//! 与模式 P 的差异: 总线是单例的一个实例, 由 [`super::pattern_p_singleton`] 启动初始化。

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::frb_generated::StreamSink;

use super::common::TransportError;

#[derive(Debug, Clone)]
pub struct TransportSampleEventDto {
    pub topic: String,
    pub payload: String,
    pub timestamp_ms: i64,
}

const BUS_CAPACITY: usize = 128;
/// 系统事件: 订阅者落后于发布者时, 用此 topic 通知 Dart 端。
pub const LAGGED_TOPIC: &str = "__lagged__";

static BUS: OnceLock<broadcast::Sender<TransportSampleEventDto>> = OnceLock::new();
/// 每个 topic 保留最新一条; 新订阅者依此构造 snapshot。
static SNAPSHOT: OnceLock<Mutex<HashMap<String, TransportSampleEventDto>>> = OnceLock::new();

fn bus() -> &'static broadcast::Sender<TransportSampleEventDto> {
    BUS.get_or_init(|| {
        let (tx, _rx) = broadcast::channel(BUS_CAPACITY);
        tx
    })
}

fn snapshot() -> &'static Mutex<HashMap<String, TransportSampleEventDto>> {
    SNAPSHOT.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Rust 内任意地方调用 (含其它 pattern 文件)。Dart 端不需要直接调用此函数。
pub fn publish_event(topic: String, payload: String) -> Result<(), TransportError> {
    if topic.is_empty() {
        return Err(TransportError::invalid_argument("topic required"));
    }
    if topic == LAGGED_TOPIC {
        return Err(TransportError::invalid_argument(
            "topic name reserved for system",
        ));
    }
    let ev = TransportSampleEventDto {
        topic: topic.clone(),
        payload,
        timestamp_ms: now_ms(),
    };
    if let Ok(mut snap) = snapshot().lock() {
        snap.insert(topic, ev.clone());
    }
    // 没订阅者时 send 返回 Err; 不视为错误。
    let _ = bus().send(ev);
    Ok(())
}

/// Dart 订阅。每次调用新建一个独立的接收端 (broadcast 语义)。
///
/// `include_snapshot=true` 时, 先回放 snapshot (每个 topic 最新一条), 再衔接实时流。
pub async fn subscribe_events(
    sink: StreamSink<TransportSampleEventDto>,
    include_snapshot: bool,
) -> Result<(), TransportError> {
    let mut rx = bus().subscribe();

    if include_snapshot {
        let snap = snapshot()
            .lock()
            .map(|m| m.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for ev in snap {
            if sink.add(ev).is_err() {
                return Ok(());
            }
        }
    }

    loop {
        match rx.recv().await {
            Ok(ev) => {
                if sink.add(ev).is_err() {
                    return Ok(());
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                let warn = TransportSampleEventDto {
                    topic: LAGGED_TOPIC.into(),
                    payload: format!("{skipped} events dropped"),
                    timestamp_ms: now_ms(),
                };
                if sink.add(warn).is_err() {
                    return Ok(());
                }
            }
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

/// 清空快照。仅供测试 / 维护; 不影响订阅者。
pub async fn transport_sample_event_bus_clear_snapshot() -> Result<u32, TransportError> {
    let mut snap = snapshot()
        .lock()
        .map_err(|_| TransportError::internal("snapshot poisoned"))?;
    let n = snap.len() as u32;
    snap.clear();
    Ok(n)
}
