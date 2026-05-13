//! 场景关键词: 纯计算 / 无 IO / < 100us / 同步返回 → 选我。
//!
//! 模式 A: 同步纯函数。
//!
//! 何时用:
//! - 输入输出都是值类型 (数字、短字符串、小 Vec)。
//! - 不访问数据库、文件、网络、锁。
//! - 运行时间是确定性的微秒量级。
//!
//! 何时不用:
//! - 函数体内任何一行可能阻塞 → 改用 [`super::pattern_b_async`]。
//! - 输入或输出是大数组 → 改用 [`super::pattern_g_bytes`]（零拷贝）。
//!
//! FRB 注解 `#[frb(sync)]` 让 Dart 侧直接得到同步函数（不返回 Future）。

use flutter_rust_bridge::frb;

use super::common::TransportError;

/// 示例: 两个数的加法。生产中应是更有意义的纯计算（哈希、几何变换等）。
#[frb(sync)]
pub fn transport_sample_add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

/// 带校验的同步函数: 演示如何返回 `Result<T, TransportError>`。
///
/// FRB 会把 `Result` 转成 Dart 侧的抛出异常。
#[frb(sync)]
pub fn transport_sample_clamp_percent(value: f64) -> Result<f64, TransportError> {
    if value.is_nan() {
        return Err(TransportError::invalid_argument("value is NaN"));
    }
    Ok(value.clamp(0.0, 1.0))
}
