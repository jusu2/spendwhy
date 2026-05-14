//! # Notes use cases (命令侧)
//!
//! 与技术无关的编排层。Use case 只依赖它实际需要的 Port 能力
//! ([`NoteReader`]、[`NoteWriter`]、[`OutboxSink`]、[`Clock`]), 类型系统会
//! 拦下连错适配器的情形。
//!
//! 每个 use case 产生的 [`NoteEvent`] 都在返回前追加到 [`OutboxSink`]。
//! 时间通过 [`Clock`] 注入, 测试可以用 [`FixedClock`] 拿到确定性。
//!
//! [`FixedClock`]: archforge_kernel::FixedClock
//! [`NoteEvent`]: archforge_contract_notes::NoteEvent

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_notes::{
    ArchiveNoteCmd, CreateNoteCmd, EditNoteCmd, NoteDto, NoteId, NoteReader, NoteWriter,
    RestoreNoteCmd,
};
use archforge_domain_notes::Note;
use archforge_kernel::{AppError, Clock, Context, OutboxSink, Result};

/// 新建笔记。
pub async fn create_note<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    ctx: &Context,
    cmd: CreateNoteCmd,
) -> Result<NoteDto>
where
    R: NoteWriter + ?Sized,
{
    let now = clock.now();
    let (note, event) = Note::create(cmd.title, cmd.body, cmd.tags, now);
    let dto = note.to_dto();
    repo.insert(ctx, &dto).await?;
    // Outbox 在写入成功**后**追加。富一点的实现会把它放进同一个 UoW
    // 事务里 —— 当前 phase 不引入事务抽象。
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// 编辑笔记 (乐观并发)。
pub async fn edit_note<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    ctx: &Context,
    cmd: EditNoteCmd,
) -> Result<NoteDto>
where
    R: NoteReader + NoteWriter + ?Sized,
{
    let existing = repo
        .find_by_id(ctx, &cmd.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("note {}", cmd.id)))?;
    let expected_version = existing.version;
    let mut note = Note::rehydrate(existing)?;
    let event = note.edit(cmd.title, cmd.body, cmd.tags, clock.now())?;
    let dto = note.to_dto();
    repo.update(ctx, &dto, expected_version).await?;
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// 归档笔记。
pub async fn archive_note<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    ctx: &Context,
    cmd: ArchiveNoteCmd,
) -> Result<NoteDto>
where
    R: NoteReader + NoteWriter + ?Sized,
{
    let existing = repo
        .find_by_id(ctx, &cmd.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("note {}", cmd.id)))?;
    let expected_version = existing.version;
    let mut note = Note::rehydrate(existing)?;
    let event = note.archive(clock.now())?;
    let dto = note.to_dto();
    repo.update(ctx, &dto, expected_version).await?;
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// 恢复笔记 (反归档)。
pub async fn restore_note<R>(
    repo: &R,
    outbox: &dyn OutboxSink,
    clock: &dyn Clock,
    ctx: &Context,
    cmd: RestoreNoteCmd,
) -> Result<NoteDto>
where
    R: NoteReader + NoteWriter + ?Sized,
{
    let existing = repo
        .find_by_id(ctx, &cmd.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("note {}", cmd.id)))?;
    let expected_version = existing.version;
    let mut note = Note::rehydrate(existing)?;
    let event = note.restore(clock.now())?;
    let dto = note.to_dto();
    repo.update(ctx, &dto, expected_version).await?;
    outbox.append(ctx, &event).await?;
    Ok(dto)
}

/// 按 id 查找; 不存在返回 `Ok(None)` (不是 NotFound 错误)。
pub async fn find_note_by_id<R>(repo: &R, ctx: &Context, id: NoteId) -> Result<Option<NoteDto>>
where
    R: NoteReader + ?Sized,
{
    repo.find_by_id(ctx, &id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use archforge_contract_notes::{Body, NoteStatus, Tag, Title};
    use archforge_infra_notes_memory::{InMemoryNoteOutbox, InMemoryNoteRepo};
    use archforge_kernel::{FixedClock, Timestamp};

    fn fixtures() -> (InMemoryNoteRepo, InMemoryNoteOutbox, FixedClock, Context) {
        (
            InMemoryNoteRepo::new(),
            InMemoryNoteOutbox::new(),
            FixedClock::new(Timestamp::from_ms(1_000_000)),
            Context::test(),
        )
    }

    fn create_cmd(title: &str, body: &str, tags: &[&str]) -> CreateNoteCmd {
        CreateNoteCmd {
            title: Title::new(title).unwrap(),
            body: Body::new(body).unwrap(),
            tags: tags.iter().map(|s| Tag::new(*s).unwrap()).collect(),
        }
    }

    #[tokio::test]
    async fn create_then_find_round_trips_and_emits_event() {
        let (repo, outbox, clock, ctx) = fixtures();
        let dto = create_note(
            &repo,
            &outbox,
            &clock,
            &ctx,
            create_cmd("hi", "body", &["rust"]),
        )
        .await
        .unwrap();
        let again = find_note_by_id(&repo, &ctx, dto.id).await.unwrap();
        assert_eq!(again.as_ref(), Some(&dto));
        assert_eq!(dto.status, NoteStatus::Active);
        assert_eq!(outbox.snapshot().len(), 1);
        assert_eq!(outbox.snapshot()[0].event_type, "notes.note.created.v1");
    }

    #[tokio::test]
    async fn edit_advances_version_and_emits_event() {
        let (repo, outbox, clock, ctx) = fixtures();
        let dto = create_note(&repo, &outbox, &clock, &ctx, create_cmd("hi", "body", &[]))
            .await
            .unwrap();
        clock.advance_ms(10);
        let edited = edit_note(
            &repo,
            &outbox,
            &clock,
            &ctx,
            EditNoteCmd {
                id: dto.id,
                title: Some(Title::new("bye").unwrap()),
                body: None,
                tags: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(edited.title.as_str(), "bye");
        assert!(edited.version > dto.version);
        assert_eq!(outbox.snapshot().len(), 2);
    }

    #[tokio::test]
    async fn edit_missing_is_not_found() {
        let (repo, outbox, clock, ctx) = fixtures();
        let err = edit_note(
            &repo,
            &outbox,
            &clock,
            &ctx,
            EditNoteCmd {
                id: archforge_contract_notes::NoteId::new(),
                title: Some(Title::new("x").unwrap()),
                body: None,
                tags: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn archive_then_restore_cycle() {
        let (repo, outbox, clock, ctx) = fixtures();
        let dto = create_note(&repo, &outbox, &clock, &ctx, create_cmd("hi", "", &[]))
            .await
            .unwrap();
        clock.advance_ms(1);
        let archived = archive_note(&repo, &outbox, &clock, &ctx, ArchiveNoteCmd { id: dto.id })
            .await
            .unwrap();
        assert_eq!(archived.status, NoteStatus::Archived);
        clock.advance_ms(1);
        let restored = restore_note(&repo, &outbox, &clock, &ctx, RestoreNoteCmd { id: dto.id })
            .await
            .unwrap();
        assert_eq!(restored.status, NoteStatus::Active);
        assert!(restored.version > archived.version);
    }

    #[tokio::test]
    async fn double_archive_is_conflict() {
        let (repo, outbox, clock, ctx) = fixtures();
        let dto = create_note(&repo, &outbox, &clock, &ctx, create_cmd("hi", "", &[]))
            .await
            .unwrap();
        clock.advance_ms(1);
        archive_note(&repo, &outbox, &clock, &ctx, ArchiveNoteCmd { id: dto.id })
            .await
            .unwrap();
        clock.advance_ms(1);
        let err = archive_note(&repo, &outbox, &clock, &ctx, ArchiveNoteCmd { id: dto.id })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }
}
