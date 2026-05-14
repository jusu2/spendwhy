//! # JSON 文件 auth 适配器
//!
//! 跨平台、零原生依赖的 auth 上下文持久化实现。
//!
//! ## 持久性
//!
//! 写操作走 **tempfile + rename + fsync**：先把新状态写入
//! 同目录的临时文件，对该临时文件 fsync，再原子地 rename
//! 覆盖正式路径。rename 之后再 fsync 父目录，使
//! rename 本身也持久化。写入途中崩溃只会留下旧状态
//! 或新状态 —— 永不撕裂。
//!
//! ## 并发
//!
//! - **进程内**：单个 `tokio::sync::Mutex` 串行化每次
//!   load-then-store 循环。`insert/update/delete` 内部的
//!   check-then-write 是单个临界区。
//! - **进程间**：每次操作在读取前对侧车文件
//!   `<path>.lock` 申请 `fs2` 排他建议锁。两个指向
//!   同一文件的 CLI 进程会看到可串行化次序。
//!
//! ## CAS
//!
//! `update` / `delete` 遵循 `expected_version`：陈旧调用方收到
//! `AppError::Conflict`。`insert` 拒绝版本非
//! `Version::INITIAL` 的 DTO。
//!
//! 能力标记：[`Writable`]。

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

/// JSON 文件 auth 仓库。
#[derive(Clone)]
pub struct JsonFileUserRepo {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl JsonFileUserRepo {
    /// 以 `path` 为后端新建仓库。该文件在首次写入时
    /// 惰性创建。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// 正式文件的路径（主要用于诊断）。
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// 在侧车文件 `<path>.lock` 上获取跨进程排他建议锁，
    /// 然后加载并解析正式文件。Windows 上必须采用侧车
    /// 模式：`tempfile::persist` 无法 rename 覆盖任何有
    /// 打开句柄的文件，因此不能用正式文件本身作为
    /// 锁目标。
    ///
    /// 返回解析后的用户列表以及持有的锁句柄。
    /// 句柄被 drop 时锁释放。
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

    /// 通过 tempfile + fsync + rename + 父目录 fsync 原子存储。
    /// 最后通过 drop `lock_file` 释放持有的锁。
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
            // 尽力而为；Windows 下目录 fsync 是空操作。
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

    async fn delete(&self, _ctx: &Context, id: &UserId, expected_version: Version) -> Result<()> {
        let _g = self.lock.lock().await;
        let me = self.clone();
        let id = *id;
        tokio::task::spawn_blocking(move || {
            let (mut users, lock) = me.load_blocking()?;
            let idx = match users.iter().position(|u| u.id == id) {
                Some(i) => i,
                None => return me.store_blocking(&users, lock), // 幂等
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

    /// 在测试装置生命周期内拥有 TempDir 的辅助类型，避免通过
    /// `mem::forget` 泄漏。装置工厂闭包克隆其路径，但
    /// 该目录在本测试结束时被 drop（并清理）。
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
        // 一致性测试装置每条性质都构造一个新仓库；我们给
        // 每个仓库各自的 TempDir，避免泄漏并保持操作
        // 隔离。`Vec` 保证所有 TempDir 在测试结束前一直存活。
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
            // 通过引用捕获进入 async 块。
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
