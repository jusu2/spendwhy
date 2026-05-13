//! 场景关键词: 快照 / 备份点 / undo / last-N 保留 / 配置回滚 → 选我。
//!
//! 模式 D: 版本化快照。
//!
//! 每次 `take` 创建目录 `{root}/{ts_ms:013}-{label}/`, 把传入的 `files` 平铺写入。
//! `restore` 把快照内容拷回目标目录 (覆盖)。`prune(keep_last_n)` 删除多余的旧快照。
//!
//! 适用: 用户级 undo (整目录回滚)、定期 config 备份。
//! 不适用: 文件级 diff (用 git); 跨设备同步 (用模式 J)。
//!
//! 命名规则: 13 位 wall-clock 毫秒前缀, 字典序 = 时间序; label 仅供人识别,
//! 不参与排序; 同毫秒两次 take 通过序号兜底 (`-001`, `-002`)。

use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::common::{now_ms, StorageError};

const LABEL_MAX_LEN: usize = 64;

#[derive(Debug, Clone)]
pub struct StorageSampleSnapshotFile {
    /// 相对快照根的路径, e.g. "config.json" 或 "subdir/data.bin"。
    pub rel_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StorageSampleSnapshotDto {
    pub id: String,
    pub ts_ms: u64,
    pub label: String,
    pub file_count: u64,
    pub total_bytes: u64,
}

/// 创建一个新快照。返回 snapshot id (目录名)。
pub async fn storage_sample_snapshot_take(
    root: String,
    label: String,
    files: Vec<StorageSampleSnapshotFile>,
) -> Result<StorageSampleSnapshotDto, StorageError> {
    validate_label(&label)?;
    if files.is_empty() {
        return Err(StorageError::invalid_argument(
            "snapshot must contain at least one file",
        ));
    }
    let ts = now_ms();
    let mut id = format!("{:013}-{label}", ts);
    let root_path = Path::new(&root);
    let mut dir = root_path.join(&id);
    let mut suffix = 1u32;
    while fs::metadata(&dir).await.is_ok() {
        id = format!("{:013}-{label}-{:03}", ts, suffix);
        dir = root_path.join(&id);
        suffix += 1;
        if suffix > 999 {
            return Err(StorageError::conflict("too many snapshots in same ms"));
        }
    }
    fs::create_dir_all(&dir).await?;

    let mut total: u64 = 0;
    for file in &files {
        validate_rel_path(&file.rel_path)?;
        let target = dir.join(&file.rel_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&target)
            .await?;
        f.write_all(&file.bytes).await?;
        f.sync_all().await?;
        total += file.bytes.len() as u64;
    }

    Ok(StorageSampleSnapshotDto {
        id,
        ts_ms: ts,
        label,
        file_count: files.len() as u64,
        total_bytes: total,
    })
}

/// 列出所有快照, 按时间升序。
pub async fn storage_sample_snapshot_list(
    root: String,
) -> Result<Vec<StorageSampleSnapshotDto>, StorageError> {
    let root_path = Path::new(&root);
    if !root_path.exists() {
        return Ok(vec![]);
    }
    let mut rd = fs::read_dir(root_path).await?;
    let mut out = Vec::new();
    while let Some(entry) = rd.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().await?;
        if !meta.is_dir() {
            continue;
        }
        let Some(parsed) = parse_id(&name) else {
            continue;
        };
        let (file_count, total_bytes) = scan_dir(&entry.path()).await?;
        out.push(StorageSampleSnapshotDto {
            id: name,
            ts_ms: parsed.0,
            label: parsed.1,
            file_count,
            total_bytes,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// 把指定快照的文件还原到 `dest_dir` (会先清空 dest 同名文件)。
pub async fn storage_sample_snapshot_restore(
    root: String,
    id: String,
    dest_dir: String,
) -> Result<u64, StorageError> {
    let src = Path::new(&root).join(&id);
    if !src.exists() {
        return Err(StorageError::not_found(id));
    }
    let dest = PathBuf::from(&dest_dir);
    fs::create_dir_all(&dest).await?;
    let count = copy_dir_recursive(&src, &dest).await?;
    Ok(count)
}

/// 仅保留最近 `keep` 个快照, 其余删除。返回删除数量。
pub async fn storage_sample_snapshot_prune(
    root: String,
    keep: u32,
) -> Result<u64, StorageError> {
    let list = storage_sample_snapshot_list(root.clone()).await?;
    let keep_u = keep as usize;
    if list.len() <= keep_u {
        return Ok(0);
    }
    let to_drop = list.len() - keep_u;
    let mut removed = 0u64;
    for snap in list.into_iter().take(to_drop) {
        let p = Path::new(&root).join(&snap.id);
        fs::remove_dir_all(&p).await?;
        removed += 1;
    }
    Ok(removed)
}

fn validate_label(label: &str) -> Result<(), StorageError> {
    if label.is_empty() {
        return Err(StorageError::invalid_argument("label must not be empty"));
    }
    if label.len() > LABEL_MAX_LEN {
        return Err(StorageError::invalid_argument(format!(
            "label too long (max {LABEL_MAX_LEN})"
        )));
    }
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(StorageError::invalid_argument(
            "label may contain only [A-Za-z0-9_-]",
        ));
    }
    Ok(())
}

fn validate_rel_path(p: &str) -> Result<(), StorageError> {
    if p.is_empty() {
        return Err(StorageError::invalid_argument("rel_path must not be empty"));
    }
    if p.starts_with('/') || p.contains('\\') {
        return Err(StorageError::invalid_argument(
            "rel_path must be relative, no '\\'",
        ));
    }
    // 逐段检查: 拒绝 ".."/"." 段, 拒绝空段 ("a//b"), 但允许 "foo..bar" 这种合法文件名。
    for seg in p.split('/') {
        if seg.is_empty() {
            return Err(StorageError::invalid_argument(
                "rel_path must not contain empty segments",
            ));
        }
        if seg == ".." || seg == "." {
            return Err(StorageError::invalid_argument(
                "rel_path must not contain '.' or '..' segments",
            ));
        }
    }
    Ok(())
}

fn parse_id(name: &str) -> Option<(u64, String)> {
    if name.len() < 14 {
        return None;
    }
    let (ts, rest) = name.split_at(13);
    let ts: u64 = ts.parse().ok()?;
    if !rest.starts_with('-') {
        return None;
    }
    Some((ts, rest[1..].to_string()))
}

async fn scan_dir(p: &Path) -> Result<(u64, u64), StorageError> {
    let mut stack = vec![p.to_path_buf()];
    let mut files = 0u64;
    let mut bytes = 0u64;
    while let Some(d) = stack.pop() {
        let mut rd = fs::read_dir(&d).await?;
        while let Some(entry) = rd.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                files += 1;
                bytes += meta.len();
            }
        }
    }
    Ok((files, bytes))
}

async fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<u64, StorageError> {
    let mut count = 0u64;
    let mut stack = vec![(src.to_path_buf(), dest.to_path_buf())];
    while let Some((s, d)) = stack.pop() {
        fs::create_dir_all(&d).await?;
        let mut rd = fs::read_dir(&s).await?;
        while let Some(entry) = rd.next_entry().await? {
            let meta = entry.metadata().await?;
            let new_dest = d.join(entry.file_name());
            if meta.is_dir() {
                stack.push((entry.path(), new_dest));
            } else {
                fs::copy(entry.path(), &new_dest).await?;
                count += 1;
            }
        }
    }
    Ok(count)
}
