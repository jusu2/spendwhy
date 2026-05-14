//! # Port 一致性装置
//!
//! 对 “auth `UserRepository` 必须做什么” 的单一权威。
//! 每个适配器都跑同一套用例 —— LSP 测试机械化。
//!
//! 两个入口：
//! - [`user_repo_conformance`] —— 顺序性质（insert/find/update
//!   语义、版本 CAS、email 索引交换、幂等删除等）。
//! - [`user_repo_concurrency_conformance`] —— 并发性质
//!   （对同一 email 或同一 id 的并行插入必须恰好选出一个胜者，
//!   绝不撕裂状态）。
//!
//! 二者均接受一个 *工厂* 闭包：每条性质获得一个全新的适配器，
//! 使结果相互独立且构造路径被反复执行。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod notes;

pub use notes::note_repo_conformance;

use archforge_contract_auth::{
    DisplayName, Email, UserDto, UserId, UserReader, UserWriter, Version,
};
use archforge_kernel::{AppError, Context, Timestamp};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 顺序一致性
// ---------------------------------------------------------------------------

/// 针对 `make` 产出的适配器，运行完整的顺序性质套件。
/// 一旦违例即 panic。
pub async fn user_repo_conformance<R, F, Fut>(make: F)
where
    R: UserReader + UserWriter + Send + Sync,
    F: Fn() -> Fut,
    Fut: core::future::Future<Output = R>,
{
    insert_then_find_by_id(&make().await).await;
    insert_then_find_by_email(&make().await).await;
    duplicate_email_is_conflict(&make().await).await;
    duplicate_id_is_conflict(&make().await).await;
    find_unknown_id_is_ok_none(&make().await).await;
    find_unknown_email_is_ok_none(&make().await).await;
    update_missing_is_not_found(&make().await).await;
    update_email_swaps_index(&make().await).await;
    insert_rejects_non_initial_version(&make().await).await;
    update_rejects_stale_version(&make().await).await;
    delete_is_idempotent(&make().await).await;
    delete_rejects_stale_version(&make().await).await;
}

fn sample(email: &str, name: &str, t: i64) -> UserDto {
    UserDto {
        id: UserId::new(),
        email: Email::new(email).expect("valid email fixture"),
        display_name: DisplayName::new(name).expect("valid name fixture"),
        password_hash: None,
        created_at: Timestamp::from_ms(t),
        updated_at: Timestamp::from_ms(t),
        version: Version::INITIAL,
        schema_version: 1,
    }
}

async fn insert_then_find_by_id<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u = sample("a@b", "Alice", 1);
    repo.insert(&ctx, &u).await.expect("insert");
    let back = repo
        .find_by_id(&ctx, &u.id)
        .await
        .expect("find")
        .expect("present");
    assert_eq!(back, u, "insert_then_find_by_id");
}

async fn insert_then_find_by_email<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u = sample("c@d", "Carol", 2);
    repo.insert(&ctx, &u).await.expect("insert");
    let back = repo
        .find_by_email(&ctx, &u.email)
        .await
        .expect("find")
        .expect("present");
    assert_eq!(back, u, "insert_then_find_by_email");
}

async fn duplicate_email_is_conflict<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u1 = sample("dup@x", "U1", 3);
    let u2 = UserDto {
        id: UserId::new(),
        ..u1.clone()
    };
    repo.insert(&ctx, &u1).await.expect("first insert");
    let err = repo
        .insert(&ctx, &u2)
        .await
        .expect_err("second insert must conflict");
    assert!(
        matches!(err, AppError::Conflict(_)),
        "duplicate_email_is_conflict: expected Conflict, got {err:?}"
    );
}

async fn duplicate_id_is_conflict<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u1 = sample("e@f", "E", 4);
    let u2 = UserDto {
        email: Email::new("g@h").unwrap(),
        ..u1.clone()
    };
    repo.insert(&ctx, &u1).await.expect("first insert");
    let err = repo
        .insert(&ctx, &u2)
        .await
        .expect_err("same id must conflict");
    assert!(
        matches!(err, AppError::Conflict(_)),
        "duplicate_id_is_conflict: expected Conflict, got {err:?}"
    );
}

async fn find_unknown_id_is_ok_none<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let res = repo.find_by_id(&ctx, &UserId::new()).await.expect("ok");
    assert!(res.is_none(), "missing id must return Ok(None)");
}

async fn find_unknown_email_is_ok_none<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let res = repo
        .find_by_email(&ctx, &Email::new("nobody@here").unwrap())
        .await
        .expect("ok");
    assert!(res.is_none(), "missing email must return Ok(None)");
}

async fn update_missing_is_not_found<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let mut u = sample("not@there", "X", 5);
    // 提升到非 INITIAL 版本以便调用 update；适配器必须
    // 因为该行根本不存在而拒绝。
    u.version = Version::INITIAL.next();
    let err = repo
        .update(&ctx, &u, Version::INITIAL)
        .await
        .expect_err("update missing must error");
    assert!(
        matches!(err, AppError::NotFound(_)),
        "update_missing_is_not_found: expected NotFound, got {err:?}"
    );
}

async fn update_email_swaps_index<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let mut u = sample("old@x", "U", 6);
    repo.insert(&ctx, &u).await.expect("insert");

    let new_email = Email::new("new@x").unwrap();
    let prev_version = u.version;
    u.email = new_email.clone();
    u.updated_at = Timestamp::from_ms(7);
    u.version = prev_version.next();
    repo.update(&ctx, &u, prev_version).await.expect("update");

    let by_old = repo
        .find_by_email(&ctx, &Email::new("old@x").unwrap())
        .await
        .expect("ok");
    assert!(by_old.is_none(), "old email index must be removed");

    let by_new = repo
        .find_by_email(&ctx, &new_email)
        .await
        .expect("ok")
        .expect("present");
    assert_eq!(by_new.id, u.id);
    assert_eq!(by_new.email, new_email);
}

async fn insert_rejects_non_initial_version<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let mut u = sample("v@x", "V", 8);
    u.version = Version::INITIAL.next();
    let err = repo
        .insert(&ctx, &u)
        .await
        .expect_err("non-initial version on insert must error");
    assert!(
        matches!(err, AppError::Invalid(_)),
        "insert_rejects_non_initial_version: expected Invalid, got {err:?}"
    );
}

async fn update_rejects_stale_version<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u = sample("s@x", "S", 9);
    repo.insert(&ctx, &u).await.expect("insert");

    let stale = Version::from_u64(999);
    let mut u2 = u.clone();
    u2.display_name = DisplayName::new("S2").unwrap();
    u2.version = stale.next();
    let err = repo
        .update(&ctx, &u2, stale)
        .await
        .expect_err("stale CAS must error");
    assert!(
        matches!(err, AppError::Conflict(_)),
        "update_rejects_stale_version: expected Conflict, got {err:?}"
    );
}

async fn delete_is_idempotent<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    repo.delete(&ctx, &UserId::new(), Version::INITIAL)
        .await
        .expect("delete missing must be Ok (idempotent)");
}

async fn delete_rejects_stale_version<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u = sample("d@x", "D", 10);
    repo.insert(&ctx, &u).await.expect("insert");
    let err = repo
        .delete(&ctx, &u.id, Version::from_u64(999))
        .await
        .expect_err("stale delete CAS must error");
    assert!(
        matches!(err, AppError::Conflict(_)),
        "delete_rejects_stale_version: expected Conflict, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 并发一致性
// ---------------------------------------------------------------------------

/// 性质：对同一 email（id 各异）的 N 次并发插入，
/// 必须恰好有一个胜者，其余 N-1 个观察到 `Conflict`。不撕裂状态。
///
/// 性质：对同一 id（email 各异）的 N 次并发插入，
/// 必须恰好有一个胜者，其余 N-1 个观察到 `Conflict`。
///
/// `R` 必须能廉价克隆到多个任务中；对内存适配器
/// 这是 `Arc<DashMap>` 克隆。为 spawn 我们要求 `R: Clone + 'static`。
pub async fn user_repo_concurrency_conformance<R, F, Fut>(make: F)
where
    R: UserReader + UserWriter + Clone + Send + Sync + 'static,
    F: Fn() -> Fut,
    Fut: core::future::Future<Output = R>,
{
    same_email_race_elects_one_winner(make().await).await;
    same_id_race_elects_one_winner(make().await).await;
}

async fn same_email_race_elects_one_winner<R>(repo: R)
where
    R: UserReader + UserWriter + Clone + Send + Sync + 'static,
{
    let ctx = Arc::new(Context::test());
    let n = 16usize;
    let email = Email::new("race@x").unwrap();
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        let r = repo.clone();
        let c = Arc::clone(&ctx);
        let e = email.clone();
        handles.push(tokio::spawn(async move {
            let u = UserDto {
                id: UserId::new(),
                email: e,
                display_name: DisplayName::new("R").unwrap(),
                password_hash: None,
                created_at: Timestamp::from_ms(0),
                updated_at: Timestamp::from_ms(0),
                version: Version::INITIAL,
                schema_version: 1,
            };
            r.insert(&c, &u).await
        }));
    }
    let mut wins = 0usize;
    let mut conflicts = 0usize;
    for h in handles {
        match h.await.expect("task join") {
            Ok(()) => wins += 1,
            Err(AppError::Conflict(_)) => conflicts += 1,
            Err(other) => panic!("unexpected error in same_email race: {other:?}"),
        }
    }
    assert_eq!(
        wins, 1,
        "same_email race: expected exactly one winner, got {wins}"
    );
    assert_eq!(
        conflicts,
        n - 1,
        "same_email race: expected {} conflicts, got {}",
        n - 1,
        conflicts
    );
    let found = repo
        .find_by_email(&ctx, &email)
        .await
        .expect("find")
        .expect("winner present");
    assert_eq!(found.email, email);
}

async fn same_id_race_elects_one_winner<R>(repo: R)
where
    R: UserReader + UserWriter + Clone + Send + Sync + 'static,
{
    let ctx = Arc::new(Context::test());
    let n = 16usize;
    let id = UserId::new();
    let mut handles = Vec::with_capacity(n);
    for i in 0..n {
        let r = repo.clone();
        let c = Arc::clone(&ctx);
        handles.push(tokio::spawn(async move {
            let u = UserDto {
                id,
                email: Email::new(format!("u{i}@x")).unwrap(),
                display_name: DisplayName::new("R").unwrap(),
                password_hash: None,
                created_at: Timestamp::from_ms(0),
                updated_at: Timestamp::from_ms(0),
                version: Version::INITIAL,
                schema_version: 1,
            };
            r.insert(&c, &u).await
        }));
    }
    let mut wins = 0usize;
    let mut conflicts = 0usize;
    for h in handles {
        match h.await.expect("task join") {
            Ok(()) => wins += 1,
            Err(AppError::Conflict(_)) => conflicts += 1,
            Err(other) => panic!("unexpected error in same_id race: {other:?}"),
        }
    }
    assert_eq!(
        wins, 1,
        "same_id race: expected exactly one winner, got {wins}"
    );
    assert_eq!(
        conflicts,
        n - 1,
        "same_id race: expected {} conflicts, got {}",
        n - 1,
        conflicts
    );
}
