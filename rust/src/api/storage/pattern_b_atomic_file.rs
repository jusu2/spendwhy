//! 场景关键词: 原子写 / 断电安全 / 防止半写 / write-temp + fsync + rename → 选我。
//!
//! 模式 B: 原子文件写。
//!
//! 标准三步走: 写 `path.tmp` → fsync(tmp) → rename(tmp → path) → fsync(parent_dir)。
//! 任一步骤失败, `.tmp` 留下不污染本体; 不会出现 path 半写状态。
//!
//! 适用: 单文件覆盖式写 (配置、settings.json、单一 manifest)。
//! 不适用: 多文件互相关联的事务 (用 sled 事务 / SQLite 事务)。
//!
//! 注意:
//! - rename 在 Windows / Unix 都是原子的 (针对同一卷)。跨卷不保证。
//! - fsync 默认开启; 通过 `fsync=false` 关闭可获得显著吞吐, 但放弃断电安全。
//! - 父目录 fsync 只在 Unix 必须 (确保目录项落盘); Windows 忽略。

use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::common::StorageError;

/// 原子写。`fsync=true` (推荐) 时保证断电后只可能看到旧值或新值, 不会半写。
pub async fn storage_sample_atomic_write(
    path: String,
    bytes: Vec<u8>,
    fsync: bool,
) -> Result<u64, StorageError> {
    let target = PathBuf::from(&path);
    validate_target(&target)?;

    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }

    let tmp = tmp_path(&target);

    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)
            .await?;
        f.write_all(&bytes).await?;
        if fsync {
            f.sync_all().await?;
        }
    }

    fs::rename(&tmp, &target).await.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        StorageError::from(e)
    })?;

    if fsync {
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fsync_dir(parent).await;
            }
        }
    }

    Ok(bytes.len() as u64)
}

/// 读取整个文件; 文件不存在返回 `not_found`。
pub async fn storage_sample_atomic_read(path: String) -> Result<Vec<u8>, StorageError> {
    let target = PathBuf::from(&path);
    validate_target(&target)?;
    match fs::read(&target).await {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(StorageError::not_found(path)),
        Err(e) => Err(e.into()),
    }
}

/// 仅当目标不存在时写入; 用于 first-time init。
pub async fn storage_sample_atomic_write_if_absent(
    path: String,
    bytes: Vec<u8>,
    fsync: bool,
) -> Result<bool, StorageError> {
    let target = PathBuf::from(&path);
    if fs::metadata(&target).await.is_ok() {
        return Ok(false);
    }
    storage_sample_atomic_write(path, bytes, fsync).await?;
    Ok(true)
}

fn validate_target(p: &Path) -> Result<(), StorageError> {
    if p.as_os_str().is_empty() {
        return Err(StorageError::invalid_argument("path must not be empty"));
    }
    if p.is_dir() {
        return Err(StorageError::invalid_argument("path is a directory"));
    }
    Ok(())
}

fn tmp_path(target: &Path) -> PathBuf {
    let mut tmp = target.to_path_buf();
    let name = target
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    tmp.set_file_name(format!(".{name}.tmp"));
    tmp
}

#[cfg(unix)]
async fn fsync_dir(dir: &Path) -> std::io::Result<()> {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let f = std::fs::File::open(&dir)?;
        f.sync_all()
    })
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
}

#[cfg(not(unix))]
async fn fsync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}
