//! # 内存 auth 适配器
//!
//! 基于 `DashMap` 的 `UserReader + UserWriter +
//! CredentialStore` 实现，外加一个进程内 [`OutboxSink`]。
//!
//! 原子性：insert/update/delete 使用 [`DashMap::entry`]，将
//! check-then-write 配对收敛为单个临界区。对同一 email 或同一 id
//! 的并发插入因此见到唯一胜者与若干
//! [`AppError::Conflict`]，绝不出现撕裂状态。
//!
//! 能力标记：[`Writable`] 与 [`BulkLoadable`]。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{
    CredentialStore, Email, PasswordHash, UserDto, UserEvent, UserId, UserReader, UserWriter,
    Version,
};
use archforge_kernel::{
    AppError, BulkLoadable, Context, DomainEvent, OutboxSink, Result, Writable,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::{Arc, Mutex};

/// 内存 auth 仓库。
#[derive(Clone, Default)]
pub struct InMemoryUserRepo {
    by_id: Arc<DashMap<UserId, UserDto>>,
    by_email: Arc<DashMap<Email, UserId>>,
}

impl InMemoryUserRepo {
    /// 全新、空的仓库。
    pub fn new() -> Self {
        Self::default()
    }

    /// 存储的用户数（主要用于测试/诊断）。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 当且仅当没有存储任何用户时为 `true`。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl Writable for InMemoryUserRepo {}
impl BulkLoadable for InMemoryUserRepo {}

#[async_trait]
impl UserReader for InMemoryUserRepo {
    async fn find_by_id(&self, _ctx: &Context, id: &UserId) -> Result<Option<UserDto>> {
        Ok(self.by_id.get(id).map(|e| e.value().clone()))
    }

    async fn find_by_email(&self, _ctx: &Context, email: &Email) -> Result<Option<UserDto>> {
        let id = match self.by_email.get(email) {
            Some(e) => *e.value(),
            None => return Ok(None),
        };
        Ok(self.by_id.get(&id).map(|e| e.value().clone()))
    }
}

#[async_trait]
impl UserWriter for InMemoryUserRepo {
    async fn insert(&self, _ctx: &Context, user: &UserDto) -> Result<()> {
        if user.version != Version::INITIAL {
            return Err(AppError::Invalid(format!(
                "insert: expected Version::INITIAL, got {}",
                user.version
            )));
        }

        // 先通过 `entry` 占住 email —— 同一临界区内
        // 既检查存在性又插入。避免 contains_key + insert 的
        // TOCTOU。
        use dashmap::mapref::entry::Entry;
        let email_entry = self.by_email.entry(user.email.clone());
        if matches!(email_entry, Entry::Occupied(_)) {
            return Err(AppError::Conflict(format!("email exists: {}", user.email)));
        }

        // 再占住 id 槽位。
        let id_entry = self.by_id.entry(user.id);
        if matches!(id_entry, Entry::Occupied(_)) {
            // 此时尚未插入 email —— 释放 `email_entry`
            // 是空操作，因为我们只借用了空闲槽位。
            return Err(AppError::Conflict(format!("id exists: {}", user.id)));
        }

        // 两个槽位都空闲 —— 提交。
        email_entry.insert_entry(user.id);
        id_entry.insert_entry(user.clone());
        Ok(())
    }

    async fn update(
        &self,
        _ctx: &Context,
        user: &UserDto,
        expected_version: Version,
    ) -> Result<()> {
        // 在整个临界区持有 id 行的 `entry`。
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(user.id) {
            Entry::Vacant(_) => Err(AppError::NotFound(format!("user {}", user.id))),
            Entry::Occupied(mut existing_slot) => {
                let existing = existing_slot.get().clone();
                if existing.version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for user {}: expected {}, found {}",
                        user.id, expected_version, existing.version
                    )));
                }
                if user.version <= expected_version {
                    return Err(AppError::Invalid(format!(
                        "update: new version {} must be strictly greater than expected {}",
                        user.version, expected_version
                    )));
                }

                if existing.email != user.email {
                    // 先原子地预订新 email 槽位再交换。
                    match self.by_email.entry(user.email.clone()) {
                        Entry::Occupied(holder) if *holder.get() != user.id => {
                            return Err(AppError::Conflict(format!(
                                "email exists: {}",
                                user.email
                            )));
                        }
                        Entry::Occupied(_) => {
                            // 同一 id（实际不可达，但无害）。
                        }
                        Entry::Vacant(slot) => {
                            slot.insert_entry(user.id);
                        }
                    }
                    self.by_email.remove(&existing.email);
                }
                existing_slot.insert(user.clone());
                Ok(())
            }
        }
    }

    async fn delete(&self, _ctx: &Context, id: &UserId, expected_version: Version) -> Result<()> {
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(*id) {
            Entry::Vacant(_) => Ok(()), // 幂等删除
            Entry::Occupied(slot) => {
                if slot.get().version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for user {}: expected {}, found {}",
                        id,
                        expected_version,
                        slot.get().version
                    )));
                }
                let dto = slot.remove();
                self.by_email.remove(&dto.email);
                Ok(())
            }
        }
    }
}

#[async_trait]
impl CredentialStore for InMemoryUserRepo {
    async fn set_password(
        &self,
        ctx: &Context,
        id: &UserId,
        hash: &PasswordHash,
        expected_version: Version,
    ) -> Result<()> {
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(*id) {
            Entry::Vacant(_) => Err(AppError::NotFound(format!("user {}", id))),
            Entry::Occupied(mut slot) => {
                if slot.get().version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for user {}: expected {}, found {}",
                        id,
                        expected_version,
                        slot.get().version
                    )));
                }
                let mut dto = slot.get().clone();
                dto.password_hash = Some(hash.clone());
                dto.version = expected_version.next();
                slot.insert(dto);
                let _ = ctx;
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 内存 outbox。
// ---------------------------------------------------------------------------

/// [`OutboxSink`] 的内存实现。
///
/// 将每个追加的事件记录在 `Vec` 中供测试检查。适用于
/// 单元测试；生产部署应使用持久化 outbox
///（sqlite/kafka/rabbit）。
#[derive(Clone, Default)]
pub struct InMemoryOutbox {
    inner: Arc<Mutex<Vec<RecordedEvent>>>,
}

/// 已记录事件的快照。
#[derive(Debug, Clone)]
pub struct RecordedEvent {
    /// 稳定的类型标识。
    pub event_type: &'static str,
    /// 聚合标识符。
    pub aggregate_id: String,
    /// 聚合声明该事件发生的挂钟时间。
    pub occurred_at_ms: i64,
}

impl InMemoryOutbox {
    /// 空 outbox。
    pub fn new() -> Self {
        Self::default()
    }

    /// 所有已记录事件的快照。
    pub fn snapshot(&self) -> Vec<RecordedEvent> {
        self.inner.lock().expect("outbox poisoned").clone()
    }
}

#[async_trait]
impl OutboxSink for InMemoryOutbox {
    async fn append(&self, _ctx: &Context, event: &dyn DomainEvent) -> Result<()> {
        let rec = RecordedEvent {
            event_type: event.event_type(),
            aggregate_id: event.aggregate_id(),
            occurred_at_ms: event.occurred_at().as_ms(),
        };
        self.inner.lock().expect("outbox poisoned").push(rec);
        Ok(())
    }
}

/// 重新导出事件类型，便于外部测试装置在不显式引入
/// contract crate 的情况下断言事件变体。
pub use archforge_contract_auth::UserEvent as ReExportedUserEvent;
// 当重新导出未被使用时，抑制 unused-import 警告。
#[allow(dead_code)]
fn _force_user_event_in_scope(_e: &UserEvent) {}

#[cfg(test)]
mod conformance_tests {
    use super::{InMemoryOutbox, InMemoryUserRepo};
    use archforge_kernel::{DomainEvent, OutboxSink, Timestamp};

    struct DummyEvent;
    impl DomainEvent for DummyEvent {
        fn event_type(&self) -> &'static str {
            "test.dummy.v1"
        }
        fn aggregate_id(&self) -> String {
            "agg".into()
        }
        fn occurred_at(&self) -> Timestamp {
            Timestamp::from_ms(0)
        }
    }

    #[tokio::test]
    async fn passes_port_conformance() {
        archforge_conformance::user_repo_conformance(|| async { InMemoryUserRepo::new() }).await;
    }

    #[tokio::test]
    async fn passes_concurrency_conformance() {
        archforge_conformance::user_repo_concurrency_conformance(|| async {
            InMemoryUserRepo::new()
        })
        .await;
    }

    #[tokio::test]
    async fn outbox_records_appended_events() {
        let o = InMemoryOutbox::new();
        let ctx = archforge_kernel::Context::test();
        OutboxSink::append(&o, &ctx, &DummyEvent).await.unwrap();
        assert_eq!(o.snapshot().len(), 1);
        assert_eq!(o.snapshot()[0].event_type, "test.dummy.v1");
    }
}
