//! # 内存版 notes 适配器
//!
//! 基于 `DashMap` 的 `NoteReader + NoteWriter` 实现, 以及一个进程内的
//! [`OutboxSink`]。
//!
//! 原子性: insert / update 用 [`DashMap::entry`], "检查 + 写"是一个临界区,
//! 不会出现 check-then-write 的 TOCTOU。
//!
//! 能力标记: [`Writable`] 与 [`BulkLoadable`] —— 让需要这些能力的 use case
//! 能在类型层面与本适配器对接。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_notes::{NoteDto, NoteId, NoteReader, NoteWriter, Version};
use archforge_kernel::{
    AppError, BulkLoadable, Context, DomainEvent, OutboxSink, Result, Writable,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::{Arc, Mutex};

/// 内存版笔记仓储。
#[derive(Clone, Default)]
pub struct InMemoryNoteRepo {
    by_id: Arc<DashMap<NoteId, NoteDto>>,
}

impl InMemoryNoteRepo {
    /// 新建一个空仓储。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前存储的笔记数 (主要给测试 / 诊断用)。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl Writable for InMemoryNoteRepo {}
impl BulkLoadable for InMemoryNoteRepo {}

#[async_trait]
impl NoteReader for InMemoryNoteRepo {
    async fn find_by_id(&self, _ctx: &Context, id: &NoteId) -> Result<Option<NoteDto>> {
        Ok(self.by_id.get(id).map(|e| e.value().clone()))
    }
}

#[async_trait]
impl NoteWriter for InMemoryNoteRepo {
    async fn insert(&self, _ctx: &Context, note: &NoteDto) -> Result<()> {
        if note.version != Version::INITIAL {
            return Err(AppError::Invalid(format!(
                "insert: expected Version::INITIAL, got {}",
                note.version
            )));
        }
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(note.id) {
            Entry::Occupied(_) => Err(AppError::Conflict(format!("id exists: {}", note.id))),
            Entry::Vacant(slot) => {
                slot.insert_entry(note.clone());
                Ok(())
            }
        }
    }

    async fn update(
        &self,
        _ctx: &Context,
        note: &NoteDto,
        expected_version: Version,
    ) -> Result<()> {
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(note.id) {
            Entry::Vacant(_) => Err(AppError::NotFound(format!("note {}", note.id))),
            Entry::Occupied(mut slot) => {
                let existing = slot.get().clone();
                if existing.version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for note {}: expected {}, found {}",
                        note.id, expected_version, existing.version
                    )));
                }
                if note.version <= expected_version {
                    return Err(AppError::Invalid(format!(
                        "update: new version {} must be strictly greater than expected {}",
                        note.version, expected_version
                    )));
                }
                slot.insert(note.clone());
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 内存版 outbox。
// ---------------------------------------------------------------------------

/// 内存版 [`OutboxSink`]: 把每个 append 进来的事件记到一个 `Vec`, 测试可以
/// 检视。生产环境请用持久化 outbox (sqlite / kafka / rabbit)。
#[derive(Clone, Default)]
pub struct InMemoryNoteOutbox {
    inner: Arc<Mutex<Vec<RecordedEvent>>>,
}

/// 一条已记录事件的快照。
#[derive(Debug, Clone)]
pub struct RecordedEvent {
    /// 稳定类型标识符。
    pub event_type: &'static str,
    /// 聚合 id。
    pub aggregate_id: String,
    /// 聚合声明的发生时刻 (毫秒)。
    pub occurred_at_ms: i64,
}

impl InMemoryNoteOutbox {
    /// 新建一个空 outbox。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前已记录事件的快照。
    pub fn snapshot(&self) -> Vec<RecordedEvent> {
        self.inner.lock().expect("outbox poisoned").clone()
    }
}

#[async_trait]
impl OutboxSink for InMemoryNoteOutbox {
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

#[cfg(test)]
mod tests {
    use super::*;
    use archforge_contract_notes::{Body, NoteEvent, NoteStatus, Tag, Title};
    use archforge_kernel::Timestamp;

    fn sample_dto() -> NoteDto {
        NoteDto {
            id: NoteId::new(),
            title: Title::new("hi").unwrap(),
            body: Body::new("").unwrap(),
            tags: vec![Tag::new("rust").unwrap()],
            status: NoteStatus::Active,
            created_at: Timestamp::from_ms(0),
            updated_at: Timestamp::from_ms(0),
            version: Version::INITIAL,
            schema_version: 1,
        }
    }

    #[tokio::test]
    async fn insert_then_find() {
        let repo = InMemoryNoteRepo::new();
        let ctx = Context::test();
        let dto = sample_dto();
        repo.insert(&ctx, &dto).await.unwrap();
        let found = repo.find_by_id(&ctx, &dto.id).await.unwrap();
        assert_eq!(found.as_ref(), Some(&dto));
    }

    #[tokio::test]
    async fn duplicate_insert_is_conflict() {
        let repo = InMemoryNoteRepo::new();
        let ctx = Context::test();
        let dto = sample_dto();
        repo.insert(&ctx, &dto).await.unwrap();
        let err = repo.insert(&ctx, &dto).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn insert_with_wrong_initial_version_is_invalid() {
        let repo = InMemoryNoteRepo::new();
        let ctx = Context::test();
        let mut dto = sample_dto();
        dto.version = Version::from_u64(2);
        let err = repo.insert(&ctx, &dto).await.unwrap_err();
        assert!(matches!(err, AppError::Invalid(_)));
    }

    #[tokio::test]
    async fn update_missing_is_not_found() {
        let repo = InMemoryNoteRepo::new();
        let ctx = Context::test();
        let mut dto = sample_dto();
        dto.version = Version::from_u64(2);
        let err = repo.update(&ctx, &dto, Version::INITIAL).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn stale_version_update_is_conflict() {
        let repo = InMemoryNoteRepo::new();
        let ctx = Context::test();
        let dto = sample_dto();
        repo.insert(&ctx, &dto).await.unwrap();
        let mut next = dto.clone();
        next.version = Version::from_u64(2);
        let err = repo
            .update(&ctx, &next, Version::from_u64(99))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn outbox_records_appended_events() {
        use archforge_contract_notes::{NoteArchived, NoteId as Id};
        let outbox = InMemoryNoteOutbox::new();
        let ctx = Context::test();
        let event = NoteEvent::Archived(NoteArchived {
            id: Id::new(),
            at: Timestamp::from_ms(5),
        });
        outbox.append(&ctx, &event).await.unwrap();
        let snap = outbox.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].event_type, "notes.note.archived.v1");
    }
}

#[cfg(test)]
mod conformance_tests {
    use super::InMemoryNoteRepo;

    #[tokio::test]
    async fn passes_port_conformance() {
        archforge_conformance::note_repo_conformance(|| async { InMemoryNoteRepo::new() }).await;
    }
}
