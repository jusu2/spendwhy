//! Notes `NoteRepository` 的 Port conformance 性质测试套件。
//!
//! 任何 `NoteReader + NoteWriter` 适配器都必须 round-trip 出同样的语义 ——
//! LSP 用机器化的方式验。当前 phase 只覆盖命令侧 (insert / update / find)。

use archforge_contract_notes::{
    Body, NoteDto, NoteId, NoteReader, NoteStatus, NoteWriter, Tag, Title, Version,
};
use archforge_kernel::{AppError, Context, Timestamp};

/// 顺序性质套件入口。`make` 每次返回一个**新**适配器实例, 保证性质之间相互
/// 独立; 出现违反时 panic。
pub async fn note_repo_conformance<R, F, Fut>(make: F)
where
    R: NoteReader + NoteWriter + Send + Sync,
    F: Fn() -> Fut,
    Fut: core::future::Future<Output = R>,
{
    insert_then_find_by_id(&make().await).await;
    duplicate_id_is_conflict(&make().await).await;
    find_unknown_id_is_ok_none(&make().await).await;
    update_missing_is_not_found(&make().await).await;
    insert_rejects_non_initial_version(&make().await).await;
    update_rejects_stale_version(&make().await).await;
    update_requires_strictly_greater_version(&make().await).await;
}

fn sample(t: i64) -> NoteDto {
    NoteDto {
        id: NoteId::new(),
        title: Title::new("title").expect("valid title fixture"),
        body: Body::new("").expect("valid body fixture"),
        tags: vec![Tag::new("a").unwrap()],
        status: NoteStatus::Active,
        created_at: Timestamp::from_ms(t),
        updated_at: Timestamp::from_ms(t),
        version: Version::INITIAL,
        schema_version: 1,
    }
}

async fn insert_then_find_by_id<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let dto = sample(1);
    repo.insert(&ctx, &dto).await.expect("insert");
    let got = repo.find_by_id(&ctx, &dto.id).await.expect("find");
    assert_eq!(got.as_ref(), Some(&dto), "insert/find round trip");
}

async fn duplicate_id_is_conflict<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let dto = sample(1);
    repo.insert(&ctx, &dto).await.expect("first insert");
    let err = repo.insert(&ctx, &dto).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)), "{:?}", err);
}

async fn find_unknown_id_is_ok_none<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let got = repo.find_by_id(&ctx, &NoteId::new()).await.expect("find");
    assert!(got.is_none(), "missing id must be Ok(None), not NotFound");
}

async fn update_missing_is_not_found<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let mut dto = sample(1);
    dto.version = Version::from_u64(2);
    let err = repo.update(&ctx, &dto, Version::INITIAL).await.unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)), "{:?}", err);
}

async fn insert_rejects_non_initial_version<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let mut dto = sample(1);
    dto.version = Version::from_u64(2);
    let err = repo.insert(&ctx, &dto).await.unwrap_err();
    assert!(matches!(err, AppError::Invalid(_)), "{:?}", err);
}

async fn update_rejects_stale_version<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let dto = sample(1);
    repo.insert(&ctx, &dto).await.expect("insert");
    let mut next = dto.clone();
    next.version = Version::from_u64(2);
    let err = repo
        .update(&ctx, &next, Version::from_u64(99))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)), "{:?}", err);
}

async fn update_requires_strictly_greater_version<R: NoteReader + NoteWriter>(repo: &R) {
    let ctx = Context::test();
    let dto = sample(1);
    repo.insert(&ctx, &dto).await.expect("insert");
    // 把"新版本"和"预期版本"都设为 INITIAL —— 应该被拒, 因为新版本必须严格大于。
    let err = repo.update(&ctx, &dto, Version::INITIAL).await.unwrap_err();
    assert!(matches!(err, AppError::Invalid(_)), "{:?}", err);
}
