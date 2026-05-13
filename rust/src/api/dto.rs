//! 跨边界 DTO 与转换。
//!
//! 设计原则：
//! - 所有 DTO 自带 `schema_version`，便于将来不破坏现有客户端做演进。
//! - 时间字段统一 `*_ms`，UTC 毫秒。Dart 用
//!   `DateTime.fromMillisecondsSinceEpoch(ms, isUtc: true).toLocal()` 还原。
//! - 不在 DTO 中放业务逻辑；任何转换或校验落在领域层。

use crate::domain::{Fragment, FragmentId, Recovery, Stage};
use crate::error::{AppError, AppResult};

pub const FRAGMENT_DTO_SCHEMA_VERSION: u32 = 1;
pub const RECOVERY_DTO_SCHEMA_VERSION: u32 = 1;
pub const HOME_VIEW_DTO_SCHEMA_VERSION: u32 = 1;
pub const RECORD_RECOVERY_OUTCOME_DTO_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct FragmentDto {
    pub schema_version: u32,
    pub id: String,
    pub created_at_ms: i64,
    pub intensity: u8,
    pub fade_period_days: u32,
    /// `outburst` | `recovery` | `relapse`
    pub stage: String,
}

#[derive(Debug, Clone)]
pub struct RecoveryDto {
    pub schema_version: u32,
    pub id: String,
    pub created_at_ms: i64,
    pub intensity: u8,
    pub description: String,
    pub related_fragment_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FragmentViewDto {
    pub id: String,
    pub fade_level: f64,
}

#[derive(Debug, Clone)]
pub struct HomeViewDto {
    pub schema_version: u32,
    pub fragments: Vec<FragmentViewDto>,
    pub growth_score: f64,
}

#[derive(Debug, Clone)]
pub struct RecordRecoveryOutcomeDto {
    pub schema_version: u32,
    pub recovery: RecoveryDto,
    pub fragments_to_advance: Vec<String>,
}

impl FragmentDto {
    pub(crate) fn into_domain(self) -> AppResult<Fragment> {
        if self.schema_version != FRAGMENT_DTO_SCHEMA_VERSION {
            return Err(AppError::invalid_input(
                "schema_version",
                format!(
                    "unsupported FragmentDto schema_version: {}",
                    self.schema_version
                ),
            ));
        }
        let stage = Stage::from_code(&self.stage)?;
        Fragment::try_new(
            self.id,
            self.created_at_ms,
            self.intensity,
            self.fade_period_days,
            stage,
        )
    }
}

impl RecoveryDto {
    pub(crate) fn into_domain(self) -> AppResult<Recovery> {
        if self.schema_version != RECOVERY_DTO_SCHEMA_VERSION {
            return Err(AppError::invalid_input(
                "schema_version",
                format!(
                    "unsupported RecoveryDto schema_version: {}",
                    self.schema_version
                ),
            ));
        }
        Recovery::try_new(
            self.id,
            self.created_at_ms,
            self.intensity,
            self.description,
            self.related_fragment_ids,
        )
    }
}

impl From<Recovery> for RecoveryDto {
    fn from(r: Recovery) -> Self {
        Self {
            schema_version: RECOVERY_DTO_SCHEMA_VERSION,
            id: r.id.into_string(),
            created_at_ms: r.created_at.ms(),
            intensity: r.intensity.value(),
            description: r.description.into_string(),
            related_fragment_ids: r
                .related_fragment_ids
                .into_iter()
                .map(FragmentId::into_string)
                .collect(),
        }
    }
}

/// 公开给 Dart 用于运行时校验：当前 Rust 端支持的 DTO 版本。
#[flutter_rust_bridge::frb(sync)]
pub fn supported_fragment_schema_version() -> u32 {
    FRAGMENT_DTO_SCHEMA_VERSION
}

#[flutter_rust_bridge::frb(sync)]
pub fn supported_recovery_schema_version() -> u32 {
    RECOVERY_DTO_SCHEMA_VERSION
}
