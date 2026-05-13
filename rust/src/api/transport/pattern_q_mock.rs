//! 场景关键词: Mock / 测试 / fake / 接口分层 → 选我。
//!
//! 模式 Q: 测试友好的边界。
//!
//! 推荐做法 (与手册 §6.13.17 一致):
//! 1. 真实业务逻辑放在 trait 后面; Rust 侧默认实现 = 真实, 测试时换 fake。
//! 2. Dart 侧测试 widget 时, 用 `RustLib.initMock(...)` 注入桩。
//!
//! 本文件提供一个最小 trait + fake 示例; 真正的 mock 注册见 `lib/transport/mock.dart`。

use flutter_rust_bridge::frb;

use super::common::TransportError;

#[frb(ignore)]
pub trait TransportSampleClock: Send + Sync {
    fn now_ms(&self) -> i64;
}

#[frb(ignore)]
pub struct SystemClock {}

#[frb(ignore)]
impl TransportSampleClock for SystemClock {
    fn now_ms(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

#[frb(ignore)]
pub struct FixedClock {
    pub now_ms: i64,
}

#[frb(ignore)]
impl TransportSampleClock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.now_ms
    }
}

/// 业务函数, 时钟由参数注入而非全局; 这使得单元测试零 mock 框架可写。
#[frb(ignore)]
pub fn freshness_ms(created_at_ms: i64, clock: &dyn TransportSampleClock) -> i64 {
    clock.now_ms() - created_at_ms
}

/// FRB 暴露的版本: 默认用 SystemClock。生产代码引用此入口。
pub fn transport_sample_freshness_ms(created_at_ms: i64) -> Result<i64, TransportError> {
    Ok(freshness_ms(created_at_ms, &SystemClock {}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_uses_injected_clock() {
        let clock = FixedClock { now_ms: 100 };
        assert_eq!(freshness_ms(30, &clock), 70);
    }
}
