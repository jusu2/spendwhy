//! 领域类型：Fragment 与 Stage。
//!
//! 字段全部由值对象组成，构造路径只有 `Fragment::try_new`，构造成功即合法。

use crate::domain::value::{AppTime, FadePeriodDays, FragmentId, Intensity};
use crate::error::{AppError, AppResult};

/// 碎片阶段。Code 与 Dart 端 `FragmentStage` 的 code 字符串保持一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Outburst,
    Recovery,
    Relapse,
}

impl Stage {
    pub fn code(self) -> &'static str {
        match self {
            Stage::Outburst => "outburst",
            Stage::Recovery => "recovery",
            Stage::Relapse => "relapse",
        }
    }

    pub fn from_code(code: &str) -> AppResult<Self> {
        match code {
            "outburst" => Ok(Stage::Outburst),
            "recovery" => Ok(Stage::Recovery),
            "relapse" => Ok(Stage::Relapse),
            other => Err(AppError::invalid_input(
                "stage",
                format!("unknown stage code: {other}"),
            )),
        }
    }
}

/// 领域中的碎片实体。
///
/// **不变式**（由值对象层强制）：
/// - `id` 非空
/// - `intensity` ∈ 1..=5
/// - `fade_period_days` > 0
/// - `created_at` >= 0 (UTC ms)
///
/// 因此调用方拿到 `&Fragment` 不需要再做合法性检查。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    pub id: FragmentId,
    pub created_at: AppTime,
    pub intensity: Intensity,
    pub fade_period_days: FadePeriodDays,
    pub stage: Stage,
}

impl Fragment {
    /// 解析原始字段（来自 DTO / 仓储）并返回合法 Fragment。
    pub fn try_new(
        id: impl Into<String>,
        created_at_ms: i64,
        intensity: u8,
        fade_period_days: u32,
        stage: Stage,
    ) -> AppResult<Self> {
        Ok(Self {
            id: FragmentId::try_new(id)?,
            created_at: AppTime::try_from_ms(created_at_ms)?,
            intensity: Intensity::try_new(intensity)?,
            fade_period_days: FadePeriodDays::try_new(fade_period_days)?,
            stage,
        })
    }
}
