//! 领域值对象（Value Objects）。
//!
//! 设计原则：**parse, don't validate**（Alexis King, 2019）。
//! 任何"原始数据 -> 受约束类型"的转换在构造时一次性完成，构造成功的对象
//! 在生命周期内不可能携带非法状态。整条调用链上的 `&Intensity`、`&FragmentId`
//! 都不再需要二次校验。
//!
//! 详见 [docs/adr/0003-value-objects.md](../../../docs/adr/0003-value-objects.md)。

use crate::error::{AppError, AppResult};

// === 标识符 ================================================================

/// 碎片标识符。非空字符串。
///
/// 之后若引入 UUIDv7 / ULID 规范，仅需收紧本类型的 `try_new`，
/// 调用方无需改动。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FragmentId(String);

impl FragmentId {
    pub fn try_new(raw: impl Into<String>) -> AppResult<Self> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(AppError::invalid_input("id", "fragment id is empty"));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for FragmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 恢复事件标识符。非空字符串。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecoveryId(String);

impl RecoveryId {
    pub fn try_new(raw: impl Into<String>) -> AppResult<Self> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(AppError::invalid_input("id", "recovery id is empty"));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for RecoveryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// === 标量值对象 ============================================================

/// 1..=5 的强度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Intensity(u8);

impl Intensity {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 5;

    pub fn try_new(v: u8) -> AppResult<Self> {
        if !(Self::MIN..=Self::MAX).contains(&v) {
            return Err(AppError::invalid_input(
                "intensity",
                format!("intensity must be in {}..={}", Self::MIN, Self::MAX),
            ));
        }
        Ok(Self(v))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

/// 淡化周期（天）。当前业务约定 > 0；后续如要锁定到 {180,270,365}，
/// 在此处收紧即可，无需改下游。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FadePeriodDays(u32);

impl FadePeriodDays {
    pub fn try_new(days: u32) -> AppResult<Self> {
        if days == 0 {
            return Err(AppError::invalid_input(
                "fade_period_days",
                "fade_period_days must be > 0",
            ));
        }
        Ok(Self(days))
    }

    pub fn days(self) -> u32 {
        self.0
    }
}

/// trim 后非空的文本。用于 Recovery.description 等"用户必填的一句话"字段。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonEmptyText(String);

impl NonEmptyText {
    pub fn try_new(raw: impl Into<String>, field: &'static str) -> AppResult<Self> {
        let raw = raw.into();
        if raw.trim().is_empty() {
            return Err(AppError::invalid_input(
                field,
                format!("{field} must be non-empty"),
            ));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// UTC 毫秒时间戳。约束：非负（unix 0 之前的时间不允许）。
///
/// 提取到值对象是为了：
/// 1. 让"未来需要切换到带时区的 OffsetDateTime"成为一处改动。
/// 2. 让 `f.created_at` 比 `f.created_at_ms` 更接近自然语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppTime(i64);

impl AppTime {
    pub fn try_from_ms(ms: i64) -> AppResult<Self> {
        if ms < 0 {
            return Err(AppError::invalid_input(
                "timestamp",
                "timestamp must be >= 0 (UTC ms)",
            ));
        }
        Ok(Self(ms))
    }

    pub fn ms(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_id_rejects_empty() {
        assert!(FragmentId::try_new("").is_err());
        assert!(FragmentId::try_new("ok").is_ok());
    }

    #[test]
    fn intensity_bounds() {
        assert!(Intensity::try_new(0).is_err());
        assert!(Intensity::try_new(6).is_err());
        assert_eq!(Intensity::try_new(3).unwrap().value(), 3);
    }

    #[test]
    fn fade_period_days_rejects_zero() {
        assert!(FadePeriodDays::try_new(0).is_err());
        assert_eq!(FadePeriodDays::try_new(270).unwrap().days(), 270);
    }

    #[test]
    fn non_empty_text_trims() {
        assert!(NonEmptyText::try_new("   ", "x").is_err());
        assert_eq!(NonEmptyText::try_new("hi", "x").unwrap().as_str(), "hi");
    }

    #[test]
    fn app_time_rejects_negative() {
        assert!(AppTime::try_from_ms(-1).is_err());
        assert_eq!(AppTime::try_from_ms(0).unwrap().ms(), 0);
    }
}
