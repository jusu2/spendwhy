//! Notes 切片端到端演示。
//!
//! ```text
//! cargo run -p notes-cli
//! ```
//!
//! 业务流程: 建 → 编辑 → 归档 → 恢复 → 列事件。所有调用都走 use case 层,
//! 后面挂的是内存适配器; 换成 sqlite/jsonfile 只需要换 `make_repo`。

#![forbid(unsafe_code)]

use anyhow::{Context as _, Result};
use archforge_app_notes::{archive_note, create_note, edit_note, find_note_by_id, restore_note};
use archforge_contract_notes::{
    ArchiveNoteCmd, Body, CreateNoteCmd, EditNoteCmd, RestoreNoteCmd, Tag, Title,
};
use archforge_infra_notes_memory::{InMemoryNoteOutbox, InMemoryNoteRepo};
use archforge_kernel::{Context, SystemClock};

#[tokio::main]
async fn main() -> Result<()> {
    let repo = InMemoryNoteRepo::new();
    let outbox = InMemoryNoteOutbox::new();
    let clock = SystemClock;
    let ctx = Context::new();

    println!("[notes-cli] backend = memory");

    let created = create_note(
        &repo,
        &outbox,
        &clock,
        &ctx,
        CreateNoteCmd {
            title: Title::new("Hello ArchForge").context("invalid title")?,
            body: Body::new("第一条笔记。").context("invalid body")?,
            tags: vec![Tag::new("demo").context("invalid tag")?],
        },
    )
    .await?;
    println!(
        "[notes-cli] created id={} version={} status={:?}",
        created.id, created.version, created.status,
    );

    let edited = edit_note(
        &repo,
        &outbox,
        &clock,
        &ctx,
        EditNoteCmd {
            id: created.id,
            title: Some(Title::new("Hello ArchForge (edited)").unwrap()),
            body: None,
            tags: Some(vec![Tag::new("demo").unwrap(), Tag::new("edited").unwrap()]),
        },
    )
    .await?;
    println!(
        "[notes-cli] edited  version={} tags={:?}",
        edited.version,
        edited.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
    );

    let archived = archive_note(
        &repo,
        &outbox,
        &clock,
        &ctx,
        ArchiveNoteCmd { id: created.id },
    )
    .await?;
    println!(
        "[notes-cli] archived version={} status={:?}",
        archived.version, archived.status,
    );

    let restored = restore_note(
        &repo,
        &outbox,
        &clock,
        &ctx,
        RestoreNoteCmd { id: created.id },
    )
    .await?;
    println!(
        "[notes-cli] restored version={} status={:?}",
        restored.version, restored.status,
    );

    let again = find_note_by_id(&repo, &ctx, created.id).await?;
    assert_eq!(again.as_ref(), Some(&restored));
    println!("[notes-cli] find_by_id round-trip OK");

    let events = outbox.snapshot();
    println!("[notes-cli] outbox recorded {} event(s):", events.len());
    for e in &events {
        println!(
            "  - {} @ {}ms (aggregate {})",
            e.event_type, e.occurred_at_ms, e.aggregate_id
        );
    }
    Ok(())
}
