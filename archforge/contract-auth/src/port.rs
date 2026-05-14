//! Port trait: 读和写 capability 分开, 以便 use case 仅 bound 在它需要
//! 的那部分上。
//!
//! `UserWriter` 上的所有写操作都使用**乐观并发控制 (OCC)**, 通过
//! [`crate::Version`] 实现: 调用方传入它期望的当前版本, adapter 用
//! [`AppError::Conflict`] 拒绝过时写入。

use crate::types::{Email, PasswordHash, UserDto, UserId, Version};
use archforge_kernel::{Context, Result};
use async_trait::async_trait;

/// auth 用户的读 capability。
#[async_trait]
pub trait UserReader: Send + Sync {
    /// 按主键 id 查找用户。
    ///
    /// `Ok(None)` **不是**错误 —— 用户单纯不存在。实现不得对缺失行返回
    /// `AppError::NotFound`。
    async fn find_by_id(&self, ctx: &Context, id: &UserId) -> Result<Option<UserDto>>;

    /// 按邮箱查找用户。
    ///
    /// 与 [`find_by_id`] 同样的 `Ok(None)` 约定。
    async fn find_by_email(&self, ctx: &Context, email: &Email) -> Result<Option<UserDto>>;
}

/// auth 用户的写 capability, 带乐观并发。
#[async_trait]
pub trait UserWriter: Send + Sync {
    /// 插入新用户。DTO 的 `version` 必须等于 [`Version::INITIAL`]。
    ///
    /// - 成功返回 `Ok(())`。
    /// - 若 `id` 或 `email` 已存在则 `AppError::Conflict`。
    /// - 若 `version != Version::INITIAL` 则 `AppError::Invalid`。
    /// - 后端瞬时错误返回 `AppError::Unavailable`。
    ///
    /// 当 `ctx.idempotency_key` 存在时, adapter 须遵守它: 对*相同* DTO
    /// 用同一 key 重试返回 `Ok(())` 而非 `Conflict`; 对*不同* DTO 仍返回
    /// `Conflict`。
    async fn insert(&self, ctx: &Context, user: &UserDto) -> Result<()>;

    /// 更新已存在的用户。调用方传入它读到的版本 (`expected_version`);
    /// adapter 仅在存储行的版本匹配时才落库。
    ///
    /// 成功后, 存储行的版本变为 `user.version` (调用方应已将其设置为
    /// `expected_version.next()`)。
    ///
    /// - 成功返回 `Ok(())`。
    /// - `id` 不存在返回 `AppError::NotFound`。
    /// - 存储版本与 `expected_version` 不一致返回 `AppError::Conflict`
    ///   (防止 lost update)。
    /// - 新邮箱与他人冲突返回 `AppError::Conflict`。
    async fn update(&self, ctx: &Context, user: &UserDto, expected_version: Version) -> Result<()>;

    /// 按 id 删除用户, 带版本检查。
    ///
    /// - 成功返回 `Ok(())` (幂等: 删除不存在的 id 也返回 Ok)。
    /// - 存储版本与 `expected_version` 不一致返回 `AppError::Conflict`。
    async fn delete(&self, ctx: &Context, id: &UserId, expected_version: Version) -> Result<()>;
}

/// 同时支持读写两半的 adapter 用的便捷 supertrait。
pub trait UserRepository: UserReader + UserWriter {}
impl<T: UserReader + UserWriter> UserRepository for T {}

/// 密码存储 Port。
///
/// 与 `UserWriter` 分开, 这样 adapter 可独立实现 —— 例如未来一个只读的
/// LDAP adapter 可只实现 [`UserReader`], 不必声称自己存密码。
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// 为用户持久化 (或替换) 密码 hash。
    ///
    /// 用户不存在时返回 `AppError::NotFound`。像 [`UserWriter::update`]
    /// 一样基于 `expected_version` 做 CAS。
    async fn set_password(
        &self,
        ctx: &Context,
        id: &UserId,
        hash: &PasswordHash,
        expected_version: Version,
    ) -> Result<()>;
}
