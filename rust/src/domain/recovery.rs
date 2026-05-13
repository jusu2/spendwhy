//! 领域类型：Recovery。
//!
//! 字段全部由值对象组成，构造路径只有 `Recovery::try_new`，构造成功即合法。

use crate::domain::value::{AppTime, FragmentId, Intensity, NonEmptyText, RecoveryId};
use crate::error::AppResult;

/// 一次"恢复事件"。
///
/// **不变式**（由值对象层强制）：
/// - `id` 非空
/// - `intensity` ∈ 1..=5
/// - `description` trim 后非空
/// - `related_fragment_ids` 中每个 id 非空（来自 `FragmentId`）
/// - `created_at` >= 0 (UTC ms)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    pub id: RecoveryId,
    pub created_at: AppTime,
    pub intensity: Intensity,
    pub description: NonEmptyText,
    pub related_fragment_ids: Vec<FragmentId>,
}

impl Recovery {
    pub fn try_new(
        id: impl Into<String>,
        created_at_ms: i64,
        intensity: u8,
        description: impl Into<String>,
        related_fragment_ids: Vec<String>,
    ) -> AppResult<Self> {
        let ids = related_fragment_ids
            .into_iter()
            .map(FragmentId::try_new)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Self {
            id: RecoveryId::try_new(id)?,
            created_at: AppTime::try_from_ms(created_at_ms)?,
            intensity: Intensity::try_new(intensity)?,
            description: NonEmptyText::try_new(description, "description")?,
            related_fragment_ids: ids,
        })
    }
}
