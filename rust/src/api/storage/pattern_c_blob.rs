//! 场景关键词: 大对象 / blob / 内容寻址 / sha256 / 去重 → 选我。
//!
//! 模式 C: Blob 内容寻址存储。
//!
//! 文件名由内容 sha256 派生 (前 32 hex = 16 字节 = 128bit, 碰撞概率可忽略)。
//! 相同内容只存一份, 天然去重。读取自带 sha256 校验, 检测磁盘位翻转。
//!
//! 写入流程: 计算 sha256 → 已存在则跳过 → 否则原子写 (复用模式 B 思路)。
//!
//! 适用: 图像、附件、不可变数据块、CRDT 状态快照。
//! 不适用: 经常更新的小数据 (用 sled / 模式 F)。

use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::common::{sha256_hex, StorageError, StorageSampleBlobReceiptDto};

const ID_PREFIX_LEN: usize = 32; // 32 hex = 16 字节, 用作目录/文件名

/// 写入 blob; 若已存在则跳过 (内容相同, 内容寻址保证)。
pub async fn storage_sample_blob_put(
    root: String,
    content: Vec<u8>,
) -> Result<StorageSampleBlobReceiptDto, StorageError> {
    let full = sha256_hex(&content);
    let id: String = full.chars().take(ID_PREFIX_LEN).collect();
    let target = blob_path(&root, &id)?;

    if fs::metadata(&target).await.is_ok() {
        return Ok(StorageSampleBlobReceiptDto {
            id,
            size_bytes: content.len() as u64,
            sha256_hex: full,
        });
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }

    let tmp = {
        let mut t = target.clone();
        t.set_extension("tmp");
        t
    };
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)
            .await?;
        f.write_all(&content).await?;
        f.sync_all().await?;
    }
    fs::rename(&tmp, &target).await?;

    Ok(StorageSampleBlobReceiptDto {
        id,
        size_bytes: content.len() as u64,
        sha256_hex: full,
    })
}

/// 读取并自动校验 sha256; 不一致返回 `corrupted`。
pub async fn storage_sample_blob_get(
    root: String,
    id: String,
) -> Result<Vec<u8>, StorageError> {
    validate_id(&id)?;
    let target = blob_path(&root, &id)?;
    let bytes = match fs::read(&target).await {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(StorageError::not_found(id));
        }
        Err(e) => return Err(e.into()),
    };
    let actual = sha256_hex(&bytes);
    if !actual.starts_with(&id) {
        return Err(StorageError::corrupted(format!(
            "blob {id}: sha256 mismatch"
        )));
    }
    Ok(bytes)
}

pub async fn storage_sample_blob_exists(
    root: String,
    id: String,
) -> Result<bool, StorageError> {
    validate_id(&id)?;
    let target = blob_path(&root, &id)?;
    Ok(fs::metadata(&target).await.is_ok())
}

pub async fn storage_sample_blob_delete(
    root: String,
    id: String,
) -> Result<bool, StorageError> {
    validate_id(&id)?;
    let target = blob_path(&root, &id)?;
    match fs::remove_file(&target).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn blob_path(root: &str, id: &str) -> Result<PathBuf, StorageError> {
    if root.is_empty() {
        return Err(StorageError::invalid_argument("root must not be empty"));
    }
    // 两层分桶 (前 2 hex 作目录) 防单目录文件数爆炸。
    let bucket = &id[..2.min(id.len())];
    Ok(Path::new(root).join(bucket).join(id))
}

fn validate_id(id: &str) -> Result<(), StorageError> {
    if id.len() != ID_PREFIX_LEN {
        return Err(StorageError::invalid_argument(format!(
            "blob id must be {ID_PREFIX_LEN} hex chars"
        )));
    }
    if !id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err(StorageError::invalid_argument(
            "blob id must be lowercase hex",
        ));
    }
    Ok(())
}
