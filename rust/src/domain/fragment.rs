//! 领域类型：Fragment 与 Stage。

use crate::error::{AppError, AppResult};

/// 碎片阶段。Code 与 Dart 端 `FragmentStage` 的 code 字符串保持一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// 领域中的碎片实体。时间统一 UTC 毫秒。
#[derive(Debug, Clone)]
pub struct Fragment {
    pub id: String,
    pub created_at_ms: i64,
    /// 1..=5
    pub intensity: u8,
    pub fade_period_days: u32,
    pub stage: Stage,
}

impl Fragment {
    pub fn validate(&self) -> AppResult<()> {
        if self.id.is_empty() {
            return Err(AppError::invalid_input("id", "fragment id is empty"));
        }
        if !(1..=5).contains(&self.intensity) {
            return Err(AppError::invalid_input(
                "intensity",
                "intensity must be in 1..=5",
            ));
        }
        if self.fade_period_days == 0 {
            return Err(AppError::invalid_input(
                "fade_period_days",
                "fade_period_days must be > 0",
            ));
        }
        Ok(())
    }
}
