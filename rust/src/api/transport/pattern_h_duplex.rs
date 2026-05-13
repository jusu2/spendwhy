//! 场景关键词: 双向 / 命令流 + 事件流 / REPL / 交互式 → 选我。
//!
//! 模式 H: 双向 duplex。
//!
//! FRB 不直接支持 Dart→Rust 的 stream 入参。常见做法:
//! - Dart→Rust 用 [Opaque 句柄 + 方法调用] 推送命令。
//! - Rust→Dart 用 `StreamSink` 推送事件。
//!
//! 本示例: `TransportSampleRepl` 句柄, Dart 调 `submit(cmd)` 入栈, `events` 流推回结果。
//!
//! 生命周期: `bind_events` 必须在 `submit` 前调用; 否则 `submit` 返回 `conflict`
//! 而不是静默丢弃。`close()` 后所有方法返回 `conflict`, 调用方需重建句柄。

use std::sync::Mutex;

use crate::frb_generated::StreamSink;

use super::common::TransportError;

#[derive(Debug, Clone)]
pub struct TransportSampleReplEvent {
    pub echo_of: String,
    pub seq: u64,
}

pub struct TransportSampleRepl {
    inner: Mutex<ReplInner>,
}

struct ReplInner {
    sink: Option<StreamSink<TransportSampleReplEvent>>,
    seq: u64,
    closed: bool,
}

impl TransportSampleRepl {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ReplInner {
                sink: None,
                seq: 0,
                closed: false,
            }),
        }
    }

    /// 第一步: Dart 订阅事件流, 把 sink 注册到句柄上。重复 bind 返回 `conflict`。
    pub fn bind_events(
        &self,
        sink: StreamSink<TransportSampleReplEvent>,
    ) -> Result<(), TransportError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| TransportError::internal("lock poisoned"))?;
        if inner.closed {
            return Err(TransportError::conflict("repl already closed"));
        }
        if inner.sink.is_some() {
            return Err(TransportError::conflict("repl already bound"));
        }
        inner.sink = Some(sink);
        Ok(())
    }

    /// 第二步: Dart 发命令; Rust 回 echo 事件。
    /// 未 bind 即 submit → `conflict`; sink 已关闭 → `conflict`。
    pub fn submit(&self, command: String) -> Result<(), TransportError> {
        if command.is_empty() {
            return Err(TransportError::invalid_argument("empty command"));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| TransportError::internal("lock poisoned"))?;
        if inner.closed {
            return Err(TransportError::conflict("repl already closed"));
        }
        let Some(sink) = inner.sink.take() else {
            return Err(TransportError::conflict("repl not bound; call bind_events first"));
        };
        inner.seq = inner.seq.wrapping_add(1);
        let seq = inner.seq;
        let send_ok = sink
            .add(TransportSampleReplEvent {
                echo_of: command,
                seq,
            })
            .is_ok();
        if !send_ok {
            inner.closed = true;
            return Err(TransportError::conflict("event sink closed by dart side"));
        }
        inner.sink = Some(sink);
        Ok(())
    }

    /// 显式关闭。后续 `submit` 返回 `conflict`; 幂等。
    pub fn close(&self) -> Result<(), TransportError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| TransportError::internal("lock poisoned"))?;
        inner.closed = true;
        inner.sink = None;
        Ok(())
    }
}

impl Default for TransportSampleRepl {
    fn default() -> Self {
        Self::new()
    }
}

pub fn transport_sample_open_repl() -> TransportSampleRepl {
    TransportSampleRepl::new()
}
