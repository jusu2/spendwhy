//! Port trait: 读写能力拆开, 这样 use case 可以精确按需声明。
//!
//! `NoteWriter` 上所有写操作都用 [`crate::Version`] 做**乐观并发控制**:
//! 调用方传入它期望当前是的版本, 适配器在写入前与库内版本比对, 不一致则
//! 返回 [`AppError::Conflict`]。

use crate::types::{NoteDto, NoteId, Version};
use archforge_kernel::{Context, Result};
use async_trait::async_trait;

/// 笔记的读能力。
#[async_trait]
pub trait NoteReader: Send + Sync {
    /// 按主键查找。
    ///
    /// `Ok(None)` **不是**错误 —— 表示该 id 不存在。实现绝**不**应在缺失时
    /// 返回 `AppError::NotFound` (那是 use case 层的语义)。
    async fn find_by_id(&self, ctx: &Context, id: &NoteId) -> Result<Option<NoteDto>>;
}

/// 笔记的写能力, 带乐观并发。
#[async_trait]
pub trait NoteWriter: Send + Sync {
    /// 插入新笔记。DTO 的 `version` 必须等于 [`Version::INITIAL`]。
    ///
    /// - `Ok(())` 表示成功。
    /// - `AppError::Conflict` 如果 `id` 已存在。
    /// - `AppError::Invalid` 如果 `version != Version::INITIAL`。
    /// - `AppError::Unavailable` 处理瞬态后端错误。
    ///
    /// 当 `ctx.idempotency_key` 存在时, 适配器要保证: 用**相同 key + 相同
    /// DTO** 重试得到 `Ok(())` 而不是 `Conflict`; 同一 key 下传**不同 DTO**
    /// 仍然回 `Conflict`。
    async fn insert(&self, ctx: &Context, note: &NoteDto) -> Result<()>;

    /// 覆盖已存在的笔记。调用方传它读到的版本号 (`expected_version`);
    /// 适配器只有在库内当前版本恰好等于 `expected_version` 时才落盘。
    ///
    /// 成功后库内版本变成 `note.version` (调用方期望已经把它置为
    /// `expected_version.next()`)。
    ///
    /// - `Ok(())` 成功。
    /// - `AppError::NotFound` 如果 `id` 不存在。
    /// - `AppError::Conflict` 如果库内版本与 `expected_version` 不一致
    ///   (lost update 防护)。
    async fn update(&self, ctx: &Context, note: &NoteDto, expected_version: Version) -> Result<()>;
}

/// 同时支持读写的便捷超 trait。
pub trait NoteRepository: NoteReader + NoteWriter {}
impl<T: NoteReader + NoteWriter> NoteRepository for T {}
