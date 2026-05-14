//! 不透明、可序列化的时间戳。
//!
//! ArchForge 刻意**不**让 `std::time::SystemTime` 或 `chrono`/`time`
//! 类型跨越 Port 边界。这样底层时间库变更时契约保持稳定, 也让 DTO 能
//! 透明地经 JSON/protobuf 往返, 无需特例编码器。
//!
//! `Timestamp::now_from_system()` 是 kernel 中*唯一*接触 `std::time`
//! 的地方。应用代码应改为依赖 [`crate::Clock`]。

use serde::{Deserialize, Serialize};

/// 自 Unix epoch (1970-01-01T00:00:00Z) 起的毫秒数。
///
/// 封装 `i64`; 支持负值以表示历史时间戳。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// **内部使用** — 供 [`crate::SystemClock`] 调用。应用代码必须改走
    /// `&dyn Clock` 路径, 这样测试才能注入 [`crate::FixedClock`]。
    pub(crate) fn now_from_system() -> Self {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0);
        Self(ms)
    }

    /// 用显式毫秒构造。对测试和 fixture 友好的 `const`。
    pub const fn from_ms(ms: i64) -> Self {
        Self(ms)
    }

    /// 内部毫秒值。
    pub const fn as_ms(&self) -> i64 {
        self.0
    }

    /// 以毫秒做饱和加法。
    pub const fn saturating_add_ms(self, ms: i64) -> Self {
        Self(self.0.saturating_add(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_from_system_is_after_2020() {
        let now = Timestamp::now_from_system();
        assert!(now.as_ms() > 1_577_836_800_000);
    }

    #[test]
    fn ord_is_natural() {
        let a = Timestamp::from_ms(100);
        let b = Timestamp::from_ms(200);
        assert!(a < b);
    }

    #[test]
    fn serde_is_transparent() {
        let t = Timestamp::from_ms(42);
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, "42");
        let back: Timestamp = serde_json::from_str("42").unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn saturating_add_does_not_overflow() {
        let t = Timestamp::from_ms(i64::MAX - 1);
        assert_eq!(t.saturating_add_ms(100).as_ms(), i64::MAX);
    }
}
