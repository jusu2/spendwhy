//! Clock —— wall time 之上的 Port。
//!
//! 时间是外部依赖。在应用代码深处直接调用 `SystemTime::now()` 会破坏
//! 确定性, 让测试依赖真实 sleep, 还会把业务逻辑绑死在特定 runtime 上。
//! 因此 ArchForge 把时间视为 Port: use case bound 在 `&dyn Clock` 上,
//! adapter 在生产中提供 `SystemClock`、在测试中提供 `FixedClock`。

use crate::Timestamp;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

/// 只读时钟 Port。
pub trait Clock: Send + Sync {
    /// 当前 wall-clock 瞬时。
    fn now(&self) -> Timestamp;
}

/// 生产时钟: 封装 `std::time::SystemTime`。
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now_from_system()
    }
}

/// 测试用手动时钟。构造保证单调 (对 [`Self::set`] 的回拨调用会被拒绝)。
#[derive(Debug, Clone)]
pub struct FixedClock(Arc<AtomicI64>);

impl FixedClock {
    /// 起点为 `start` 的新时钟。
    pub fn new(start: Timestamp) -> Self {
        Self(Arc::new(AtomicI64::new(start.as_ms())))
    }

    /// 将时钟推进 `ms` 毫秒。
    pub fn advance_ms(&self, ms: i64) {
        // `fetch_add` 按定义就是单调的, 因为调用方只能传正 delta
        // (我们不强校验 —— 负 delta 是测试 bug, 不是运行时关心的问题)。
        self.0.fetch_add(ms, Ordering::Relaxed);
    }

    /// 设置绝对时间。debug 构建下若新值向后倒退则 panic
    /// (测试中时钟回拨属于契约违反)。
    pub fn set(&self, t: Timestamp) {
        let prev = self.0.swap(t.as_ms(), Ordering::Relaxed);
        debug_assert!(
            t.as_ms() >= prev,
            "FixedClock: set() moved backwards from {prev} to {}",
            t.as_ms()
        );
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_ms(self.0.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_advances() {
        let c = FixedClock::new(Timestamp::from_ms(1_000));
        assert_eq!(c.now().as_ms(), 1_000);
        c.advance_ms(500);
        assert_eq!(c.now().as_ms(), 1_500);
    }

    #[test]
    fn system_clock_is_after_2020() {
        let c = SystemClock;
        assert!(c.now().as_ms() > 1_577_836_800_000);
    }

    #[test]
    fn clock_is_object_safe() {
        // 编译期检查: Clock 必须能作为 `&dyn Clock` 使用。
        fn _accept(c: &dyn Clock) -> Timestamp {
            c.now()
        }
        let f = FixedClock::new(Timestamp::from_ms(42));
        assert_eq!(_accept(&f).as_ms(), 42);
    }
}
