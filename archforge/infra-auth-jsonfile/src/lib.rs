//! # JSON-file auth adapter
//!
//! Cross-platform, zero-native-dep persistence for the auth context.
//!
//! ## Durability
//!
//! Writes go through **tempfile + rename + fsync**: the new state is written
//! to a sibling temp file, the temp file is fsynced, then atomically renamed
//! over the canonical path. After rename we fsync the parent directory so
//! the rename itself is durable. A crash mid-write leaves either the old
//! state or the new — never a torn file.
//!
//! ## Concurrency
//!
//! - **Intra-process**: a single `tokio::sync::Mutex` serialises every
//!   load-then-store cycle. The check-then-write inside
//!   `insert/update/delete` is one critical section.
//! - **Inter-process**: each operation acquires an exclusive `fs2` advisory
//!   lock on a sidecar `<path>.lock` file before reading. Two CLI processes
//!   pointed at the same file see a serialisable order.
//!
//! ## CAS
//!
//! `update` / `delete` honour `expected_version`: stale callers receive
//! `AppError::Conflict`. `insert` rejects any DTO whose version isn't
//! `Version::INITIAL`.
//!
//! Capability markers: [`Writable`].

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{
    CredentialStore, Email, PasswordHash, UserDto, UserId, UserReader, UserWriter, Version,
};
use archforge_kernel::{AppError, Context, Result, Writable};
use async_trait::async_trait;
use fs2::FileExt;
use std::path::PathBuf;
use std::sync::Arc;
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

    /// Path to the canonical file (mostly for diagnostics).
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Acquire an exclusive cross-process advisory lock on a sidecar
    /// `<path>.lock` file, then load and parse the canonical file. The
    /// sidecar pattern is required on Windows: `tempfile::persist` cannot
    /// rename over a file that has any open handle, so we cannot use the
    /// canonical file itself as the lock target.
    ///
    /// Returns the parsed users plus the held lock handle. The lock is
    /// released when the handle is dropped.
    fn load_blocking(&self) -> Result<(Vec<UserDto>, std::fs::File)> {
        use std::fs::OpenOptions;
        use std::io::ErrorKind;

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::Unavailable(format!("mkdir: {e}")))?;
            }
        }

        let lock_path = lock_path_for(&self.path);
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| AppError::Unavailable(format!("open lock: {e}")))?;
        lock_file
            .lock_exclusive()
            .map_err(|e| AppError::Unavailable(format!("flock: {e}")))?;

        let users = match std::fs::read(&self.path) {
            Ok(b) if b.is_empty() => Vec::new(),
            Ok(b) => serde_json::from_slice(&b)
                .map_err(|e| AppError::Internal(format!("decode: {e}")))?,
            Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(AppError::Unavailable(format!("read: {e}"))),
        };
        Ok((users, lock_file))
    }

    /// Atomic store via tempfile + fsync + rename + parent fsync. Releases
    /// the held lock at the end (by dropping `lock_file`).
    fn store_blocking(&self, users: &[UserDto], lock_file: std::fs::File) -> Result<()> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let bytes = serde_json::to_vec_pretty(users)
            .map_err(|e| AppError::Internal(format!("encode: {e}")))?;

        let dir = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let mut tmp = NamedTempFile::new_in(&dir)
            .map_err(|e| AppError::Unavailable(format!("tempfile: {e}")))?;
        tmp.write_all(&bytes)
            .map_err(|e| AppError::Unavailable(format!("write tmp: {e}")))?;
        tmp.write_all(b"\n").ok();
        tmp.as_file()
            .sync_all()
            .map_err(|e| AppError::Unavailable(format!("fsync tmp: {e}")))?;

        tmp.persist(&self.path)
            .map_err(|e| AppError::Unavailable(format!("persist: {e}")))?;

        if let Ok(d) = std::fs::File::open(&dir) {
            // Best-effort; on Windows directory fsync is a no-op.
            let _ = d.sync_all();
        }

        drop(lock_file);
        Ok(())
    }
}

fn lock_path_for(path: &std::path::Path) -> std::path::PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".lock");
    std::path::PathBuf::from(p)
}

impl Writable for JsonFileUserRepo {}

#[async_trait]
impl UserReader for JsonFileUserRepo {
    async fn find_by_id(&self, _ctx: &Context, id: &UserId) -> Result<Option<UserDto>> {
        let _g = self.lock.lock().await;
        let me = self.clone();
        let id = *id;
        tokio::task::spawn_blocking(move || {
            let (users, _lock) = me.load_blocking()?;
            Ok(users.into_iter().find(|u| u.id == id))
        })
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
    }

    async fn find_by_email(&self, _ctx: &Context, email: &Email) -> Result<Option<UserDto>> {
        let _g = self.lock.lock().await;
        let me = self.clone();
        let email = email.clone();
        tokio::task::spawn_blocking(move || {
            let (users, _lock) = me.load_blocking()?;
            Ok(users.into_iter().find(|u| u.email == email))
        })
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
    }
}

#[async_trait]
impl UserWriter for JsonFileUserRepo {
    async fn insert(&self, _ctx: &Context, user: &UserDto) -> Result<()> {
        if user.version != Version::INITIAL {
            return Err(AppError::Invalid(format!(
                "insert: expected Version::INITIAL, got {}",
                user.version
            )));
        }
        let _g = self.lock.lock().await;
        let me = self.clone();
        let user = user.clone();
        tokio::task::spawn_blocking(move || {
            let (mut users, lock) = me.load_blocking()?;
            if users.iter().any(|u| u.email == user.email) {
                return Err(AppError::Conflict(format!("email exists: {}", user.email)));
            }
            if users.iter().any(|u| u.id == user.id) {
                return Err(AppError::Conflict(format!("id exists: {}", user.id)));
            }
            users.push(user);
            me.store_blocking(&users, lock)
        })
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
    }

    async fn update(
        &self,
        _ctx: &Context,
        user: &UserDto,
        expected_version: Version,
    ) -> Result<()> {
        let _g = self.lock.lock().await;
        let me = self.clone();
        let user = user.clone();
        tokio::task::spawn_blocking(move || {
            let (mut users, lock) = me.load_blocking()?;
            let idx = users
                .iter()
                .position(|u| u.id == user.id)
                .ok_or_else(|| AppError::NotFound(format!("user {}", user.id)))?;
            if users[idx].version != expected_version {
                return Err(AppError::Conflict(format!(
                    "version mismatch for user {}: expected {}, found {}",
                    user.id, expected_version, users[idx].version
                )));
            }
            if user.version <= expected_version {
                return Err(AppError::Invalid(format!(
                    "update: new version {} must be strictly greater than expected {}",
                    user.version, expected_version
                )));
            }
            if users
                .iter()
                .enumerate()
                .any(|(i, u)| i != idx && u.email == user.email)
            {
                return Err(AppError::Conflict(format!("email exists: {}", user.email)));
            }
            users[idx] = user;
            me.store_blocking(&users, lock)
        })
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
    }

    async fn delete(
        &self,
        _ctx: &Context,
        id: &UserId,
        expected_version: Version,
    ) -> Result<()> {
        let _g = self.lock.lock().await;
        let me = self.clone();
        let id = *id;
        tokio::task::spawn_blocking(move || {
            let (mut users, lock) = me.load_blocking()?;
            let idx = match users.iter().position(|u| u.id == id) {
                Some(i) => i,
                None => return me.store_blocking(&users, lock), // idempotent
            };
            if users[idx].version != expected_version {
                return Err(AppError::Conflict(format!(
                    "version mismatch for user {}: expected {}, found {}",
                    id, expected_version, users[idx].version
                )));
            }
            users.remove(idx);
            me.store_blocking(&users, lock)
        })
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
    }
}

#[async_trait]
impl CredentialStore for JsonFileUserRepo {
    async fn set_password(
        &self,
        _ctx: &Context,
        id: &UserId,
        hash: &PasswordHash,
        expected_version: Version,
    ) -> Result<()> {
        let _g = self.lock.lock().await;
        let me = self.clone();
        let id = *id;
        let hash = hash.clone();
        tokio::task::spawn_blocking(move || {
            let (mut users, lock) = me.load_blocking()?;
            let idx = users
                .iter()
                .position(|u| u.id == id)
                .ok_or_else(|| AppError::NotFound(format!("user {}", id)))?;
            if users[idx].version != expected_version {
                return Err(AppError::Conflict(format!(
                    "version mismatch for user {}: expected {}, found {}",
                    id, expected_version, users[idx].version
                )));
            }
            users[idx].password_hash = Some(hash);
            users[idx].version = expected_version.next();
            me.store_blocking(&users, lock)
        })
        .await
        .map_err(|e| AppError::Internal(format!("join: {e}")))?
    }
}

#[cfg(test)]
mod conformance_tests {
    use super::JsonFileUserRepo;
    use tempfile::TempDir;

    /// Helper that owns a TempDir for the lifetime of the test harness so we
    /// don't leak it via `mem::forget`. The harness factory closure clones
    /// the path but the dir is dropped (and cleaned) when this test ends.
    struct Fixture {
        _dir: TempDir,
        path: std::path::PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = TempDir::new().expect("tempdir");
            let path = dir.path().join("users.json");
            Self { _dir: dir, path }
        }
    }

    #[tokio::test]
    async fn passes_port_conformance() {
        // The conformance harness builds a fresh repo per property; we give
        // each one its own TempDir so nothing leaks and operations stay
        // isolated. The `Vec` keeps every TempDir alive until the test ends.
        let fixtures = std::sync::Mutex::new(Vec::<Fixture>::new());
        archforge_conformance::user_repo_conformance(|| async {
            let f = Fixture::new();
            let path = f.path.clone();
            fixtures.lock().unwrap().push(f);
            JsonFileUserRepo::new(path)
        })
        .await;
    }

    #[tokio::test]
    async fn passes_concurrency_conformance() {
        let fixtures = std::sync::Mutex::new(Vec::<Fixture>::new());
        archforge_conformance::user_repo_concurrency_conformance(|| {
            // Capture by reference into the async block.
            async {
                let f = Fixture::new();
                let path = f.path.clone();
                fixtures.lock().unwrap().push(f);
                JsonFileUserRepo::new(path)
            }
        })
        .await;
    }
}
