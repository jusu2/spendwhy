//! Capability 标记 trait。
//!
//! 零开销的标记, 让 *use case* 声明 adapter 必须提供哪些 capability。
//! 目的是把正确性推进类型系统: 需要批量加载的 use case 不可能被错配
//! 到不支持它的 adapter 上。
//!
//! ```ignore
//! pub async fn import_users<R>(repo: &R, ctx: &Context, users: Vec<UserDto>) -> Result<usize>
//! where
//!     R: UserWriter + BulkLoadable,
//! { /* ... */ }
//!
//! // Adapter 声明该 capability:
//! impl BulkLoadable for SqliteUserRepo {}
//!
//! // InMemory adapter 没有实现 BulkLoadable, 所以:
//! //    import_users(&memory_repo, &ctx, vec![...]).await
//! // 编译失败 —— 类型系统拒绝错配。
//! ```
//!
//! 这些标记刻意不带方法: **业务** capability 在 `contract-*` crate
//! 的 Port trait 中定义; 标记只表明 adapter 选择性地承担了相应的性能 /
//! 语义保证 (事务、批量、流式、…)。后果是: 新增一个标记是一行的、
//! 非破坏性的改动。

/// Adapter 仅支持读操作。
pub trait ReadOnly {}

/// Adapter 支持状态变更写入。
pub trait Writable {}

/// Adapter 对多次操作暴露事务性工作单元。
///
/// 当 use case 需要跨多次 Port 调用的 all-or-nothing 语义时 bound 在它上面。
pub trait Transactional {}

/// Adapter 能比逐行写入更高效地接收大批量。
///
/// `import_*` / `bulk_*` use case bound 在它上面。
pub trait BulkLoadable {}

/// Adapter 能以长连接流而非轮询的方式投递事件。
///
/// projector / 读模型维护类 use case bound 在它上面。
pub trait Streamable {}

#[cfg(test)]
mod tests {
    use super::*;

    // 编译期检查: 这些标记可作为 super-trait bound 使用。
    fn _accepts_writable<T: Writable>(_: &T) {}
    fn _accepts_bulk<T: Writable + BulkLoadable>(_: &T) {}

    struct Toy;
    impl Writable for Toy {}
    impl BulkLoadable for Toy {}

    #[test]
    fn capabilities_compose() {
        let t = Toy;
        _accepts_writable(&t);
        _accepts_bulk(&t);
    }
}
