//! End-to-end demo of an ArchForge auth slice.
//!
//! Choose the backend at build time via Cargo features:
//!
//! ```text
//! cargo run -p auth-cli --no-default-features --features memory-backend   -- demo
//! cargo run -p auth-cli --no-default-features --features jsonfile-backend -- demo
//! ```
//!
//! The business code in [`run`] is identical for both — only [`make_repo`]
//! changes. That is the headline property the ArchForge library delivers.

#![forbid(unsafe_code)]

use anyhow::{Context as _, Result};
use archforge_app_auth::{
    create_user, find_user_by_email, find_user_by_id, verify_password, PasswordHasher,
};
use archforge_contract_auth::{
    CreateUserCmd, DisplayName, Email, PlainPassword, VerifyPasswordCmd,
};
use archforge_infra_auth_memory::InMemoryOutbox;
use archforge_kernel::{Context, SystemClock};

#[cfg(all(feature = "memory-backend", feature = "jsonfile-backend"))]
compile_error!(
    "auth-cli: enable exactly ONE backend feature (memory-backend OR jsonfile-backend)."
);

#[cfg(not(any(feature = "memory-backend", feature = "jsonfile-backend")))]
compile_error!(
    "auth-cli: enable a backend feature (--features memory-backend or --features jsonfile-backend)."
);

#[cfg(feature = "memory-backend")]
mod backend {
    pub use archforge_infra_auth_memory::InMemoryUserRepo as Repo;
    pub fn make() -> Repo {
        Repo::new()
    }
    pub const NAME: &str = "memory";
}

#[cfg(feature = "jsonfile-backend")]
mod backend {
    pub use archforge_infra_auth_jsonfile::JsonFileUserRepo as Repo;
    pub fn make() -> Repo {
        let path = std::env::var("ARCHFORGE_AUTH_FILE")
            .unwrap_or_else(|_| "./.archforge-auth.json".to_owned());
        Repo::new(path)
    }
    pub const NAME: &str = "jsonfile";
}

#[tokio::main]
async fn main() -> Result<()> {
    let repo = backend::make();
    println!("[auth-cli] backend = {}", backend::NAME);
    run(&repo).await
}

async fn run<R>(repo: &R) -> Result<()>
where
    R: archforge_contract_auth::UserReader + archforge_contract_auth::UserWriter,
{
    let ctx = Context::new();
    let outbox = InMemoryOutbox::new();
    let clock = SystemClock;
    // Production preset is slow on cold runs; CLI demo uses test_fast for
    // snappy startup. Real services should call `PasswordHasher::production()`.
    let hasher = PasswordHasher::test_fast();

    let email = Email::new("demo@archforge.dev").context("invalid demo email")?;
    let name = DisplayName::new("Demo User").context("invalid demo name")?;
    let plain_pw = "demo-password-please-rotate";

    let dto = match find_user_by_email(repo, &ctx, email.clone()).await? {
        Some(u) => {
            println!("[auth-cli] user already exists -> looked up");
            u
        }
        None => {
            let cmd = CreateUserCmd {
                email: email.clone(),
                display_name: name,
                password: Some(PlainPassword::new(plain_pw)),
            };
            let dto = create_user(repo, &outbox, &clock, &hasher, &ctx, cmd).await?;
            println!("[auth-cli] created user");
            dto
        }
    };

    println!(
        "[auth-cli] id={}  email={}  name={}  created_at={}  version={}",
        dto.id,
        dto.email,
        dto.display_name,
        dto.created_at.as_ms(),
        dto.version,
    );

    let again = find_user_by_id(repo, &ctx, dto.id).await?;
    assert_eq!(again.as_ref(), Some(&dto));
    println!("[auth-cli] find_by_id round-trip OK");

    if dto.password_hash.is_some() {
        let auth = verify_password(
            repo,
            &outbox,
            &clock,
            &hasher,
            &ctx,
            VerifyPasswordCmd {
                email,
                password: PlainPassword::new(plain_pw),
            },
        )
        .await?;
        println!("[auth-cli] password verified for id={}", auth.user.id);
    }

    println!(
        "[auth-cli] outbox recorded {} event(s)",
        outbox.snapshot().len()
    );
    Ok(())
}
