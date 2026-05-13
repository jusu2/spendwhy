//! # JSON-file auth adapter
//!
//! Async, cross-platform, zero-native-dep persistence for the auth context.
//! Stores the full collection as a single JSON document; serialises writes
//! through an internal `tokio::sync::Mutex` so concurrent inserts/updates
//! don't tear the file.
//!
//! Intended for desktop apps, dev environments, and as the "second adapter"
//! that proves Adapter LSP via the conformance harness. Real production
//! deployments should swap in a SQLite/Postgres adapter (Step 3).

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{Email, UserDto, UserId, UserReader, UserWriter};
use archforge_kernel::{AppError, Context, Result, Writable};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

/// JSON-file auth repository.
#[derive(Clone)]
pub struct JsonFileUserRepo {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl JsonFileUserRepo {
    /// New repo backed by `path`. The file is created lazily on first write.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    async fn load(&self) -> Result<Vec<UserDto>> {
        match fs::read(&self.path).await {
            Ok(bytes) if bytes.is_empty() => Ok(Vec::new()),
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| AppError::Internal(format!("decode: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(AppError::Unavailable(format!("read: {e}"))),
        }
    }

    async fn store(&self, users: &[UserDto]) -> Result<()> {
        let mut bytes = serde_json::to_vec_pretty(users)
            .map_err(|e| AppError::Internal(format!("encode: {e}")))?;
        bytes.push(b'\n');
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| AppError::Unavailable(format!("mkdir: {e}")))?;
            }
        }
        fs::write(&self.path, &bytes)
            .await
            .map_err(|e| AppError::Unavailable(format!("write: {e}")))
    }
}

impl Writable for JsonFileUserRepo {}

#[async_trait]
impl UserReader for JsonFileUserRepo {
    async fn find_by_id(&self, _ctx: &Context, id: &UserId) -> Result<Option<UserDto>> {
        let _g = self.lock.lock().await;
        let users = self.load().await?;
        Ok(users.into_iter().find(|u| &u.id == id))
    }

    async fn find_by_email(&self, _ctx: &Context, email: &Email) -> Result<Option<UserDto>> {
        let _g = self.lock.lock().await;
        let users = self.load().await?;
        Ok(users.into_iter().find(|u| &u.email == email))
    }
}

#[async_trait]
impl UserWriter for JsonFileUserRepo {
    async fn insert(&self, _ctx: &Context, user: &UserDto) -> Result<()> {
        let _g = self.lock.lock().await;
        let mut users = self.load().await?;
        if users.iter().any(|u| u.email == user.email) {
            return Err(AppError::Conflict(format!("email exists: {}", user.email)));
        }
        if users.iter().any(|u| u.id == user.id) {
            return Err(AppError::Conflict(format!("id exists: {}", user.id)));
        }
        users.push(user.clone());
        self.store(&users).await
    }

    async fn update(&self, _ctx: &Context, user: &UserDto) -> Result<()> {
        let _g = self.lock.lock().await;
        let mut users = self.load().await?;
        let idx = users
            .iter()
            .position(|u| u.id == user.id)
            .ok_or_else(|| AppError::NotFound(format!("user {}", user.id)))?;
        if users
            .iter()
            .enumerate()
            .any(|(i, u)| i != idx && u.email == user.email)
        {
            return Err(AppError::Conflict(format!("email exists: {}", user.email)));
        }
        users[idx] = user.clone();
        self.store(&users).await
    }
}

#[cfg(test)]
mod conformance_tests {
    use super::JsonFileUserRepo;
    use tempfile::TempDir;

    #[tokio::test]
    async fn passes_port_conformance() {
        // Each invocation of the closure must return a *fresh* repo;
        // tempfile guarantees an isolated directory per construction.
        archforge_conformance::user_repo_conformance(|| async {
            let dir = TempDir::new().expect("tempdir");
            let path = dir.path().join("users.json");
            // Leak the TempDir so the file outlives the closure but cleans up
            // when the test process exits. The conformance harness performs
            // many independent operations against the same instance.
            std::mem::forget(dir);
            JsonFileUserRepo::new(path)
        })
        .await;
    }
}
