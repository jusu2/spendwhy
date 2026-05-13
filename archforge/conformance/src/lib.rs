//! # Port conformance harness
//!
//! A single source of truth for **what a `UserRepository` must do**. Every
//! adapter (`infra-auth-memory`, `infra-auth-jsonfile`, future SQLite,
//! future Postgres, …) is expected to pass [`user_repo_conformance`]
//! unchanged. This is invariant #5 of ArchForge: LSP, mechanised.
//!
//! The harness takes a *factory* closure rather than a single repo so each
//! property gets a fresh, isolated instance — guarantees independence and
//! exercises the adapter's construction path repeatedly.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{DisplayName, Email, UserDto, UserId, UserReader, UserWriter};
use archforge_kernel::{AppError, Context, Timestamp};

/// Run the full UserRepository property suite against the adapter produced
/// by `make`.
///
/// Panics on the first violation, identifying which property failed.
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
}

// --- properties --------------------------------------------------------------

fn sample(email: &str, name: &str, t: i64) -> UserDto {
    UserDto {
        id: UserId::new(),
        email: Email::new(email).expect("valid email fixture"),
        display_name: DisplayName::new(name).expect("valid name fixture"),
        created_at: Timestamp::from_ms(t),
        updated_at: Timestamp::from_ms(t),
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
    assert_eq!(
        back, u,
        "insert_then_find_by_id: read-after-write must match"
    );
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
    assert!(
        res.is_none(),
        "find_unknown_id_is_ok_none: missing rows must return Ok(None), not NotFound"
    );
}

async fn find_unknown_email_is_ok_none<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let res = repo
        .find_by_email(&ctx, &Email::new("nobody@here").unwrap())
        .await
        .expect("ok");
    assert!(
        res.is_none(),
        "find_unknown_email_is_ok_none: missing rows must return Ok(None)"
    );
}

async fn update_missing_is_not_found<R: UserReader + UserWriter>(repo: &R) {
    let ctx = Context::test();
    let u = sample("not@there", "X", 5);
    let err = repo.update(&ctx, &u).await.expect_err("update missing");
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
    u.email = new_email.clone();
    u.updated_at = Timestamp::from_ms(7);
    repo.update(&ctx, &u).await.expect("update");

    // Old email must no longer resolve to this user.
    let by_old = repo
        .find_by_email(&ctx, &Email::new("old@x").unwrap())
        .await
        .expect("ok");
    assert!(
        by_old.is_none(),
        "update_email_swaps_index: old email index must be removed"
    );

    // New email must resolve.
    let by_new = repo
        .find_by_email(&ctx, &new_email)
        .await
        .expect("ok")
        .expect("present");
    assert_eq!(by_new.id, u.id);
    assert_eq!(by_new.email, new_email);
}
