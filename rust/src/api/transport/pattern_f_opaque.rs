//! 场景关键词: 长生命周期对象 / 跨多次调用持有状态 / Session → 选我。
//!
//! 模式 F: RustOpaque 句柄。
//!
//! Dart 侧:
//! ```dart
//! final session = await transportSampleOpenSession(initial: 0);
//! await session.increment(by: 5);
//! final v = await session.snapshot();
//! await session.dispose(); // 用完显式释放
//! ```
//!
//! 实现要点 (手册 §6.5.5):
//! 1. 内部状态用 `Mutex` 保护 (FRB 不会自动同步)。
//! 2. 提供 `dispose` 入口让 Dart 主动释放; 不要塞进全局 provider。
//! 3. 不要在 Opaque 内部持有 Dart 回调而不提供取消路径 (会泄漏)。

use std::sync::Mutex;

use super::common::TransportError;

pub struct TransportSampleSession {
    state: Mutex<SessionState>,
}

struct SessionState {
    counter: i64,
    disposed: bool,
}

impl TransportSampleSession {
    /// 工厂入口: Dart 侧 `await transportSampleOpenSession(initial: 0)`。
    pub fn open(initial: i64) -> TransportSampleSession {
        TransportSampleSession {
            state: Mutex::new(SessionState {
                counter: initial,
                disposed: false,
            }),
        }
    }

    pub fn increment(&self, by: i64) -> Result<(), TransportError> {
        let mut s = self
            .state
            .lock()
            .map_err(|_| TransportError::internal("lock poisoned"))?;
        if s.disposed {
            return Err(TransportError::conflict("session disposed"));
        }
        s.counter = s.counter.wrapping_add(by);
        Ok(())
    }

    pub fn snapshot(&self) -> Result<i64, TransportError> {
        let s = self
            .state
            .lock()
            .map_err(|_| TransportError::internal("lock poisoned"))?;
        if s.disposed {
            return Err(TransportError::conflict("session disposed"));
        }
        Ok(s.counter)
    }

    /// 显式释放业务资源 (此处只是设标志)。Dart 侧 `RustOpaque` Drop 时也会回收内存。
    pub fn dispose(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.disposed = true;
        }
    }
}

/// 顶层工厂函数, 避免依赖 FRB 对 `impl` 静态方法的解析。
pub fn transport_sample_open_session(initial: i64) -> TransportSampleSession {
    TransportSampleSession::open(initial)
}
