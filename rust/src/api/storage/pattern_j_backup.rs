//! 场景关键词: 备份 / 导出 / 导入 / 跨设备迁移 / 灾难恢复 → 选我。
//!
//! 模式 J: 极简自实现归档 (无 tar 依赖)。
//!
//! 格式: NDJSON 单文件, 第一行 manifest, 后续每行一个文件条目。
//!
//! ```text
//! {"version":1,"created_ms":...,"file_count":N,"sha256":"<archive sha256 over body lines>"}
//! {"rel":"a.txt","size":12,"sha256":"...","b64":"..."}
//! {"rel":"sub/b.bin","size":...,"sha256":"...","b64":"..."}
//! ```
//!
//! - `b64` 是 base64 (standard, no padding) 编码的文件全字节。
//! - `sha256` 是该文件原始字节的 sha256 (lowercase hex)。
//! - manifest 的 `sha256` 是除 manifest 自己外所有 body 行拼接的 sha256, 用作完整性校验。
//!
//! 适用: <100MB 的少量文件备份。大数据用真正的 tar+zstd, 不在标本范围。
//!
//! 不适用: 增量备份 (这是 full snapshot); 加密备份 (用模式 I 先加密再 backup)。

use std::path::{Path, PathBuf};

use base64::Engine;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::common::{sha256_hex, StorageError};

const VERSION: u32 = 1;
const MAX_TOTAL_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct StorageSampleBackupReceiptDto {
    pub archive_path: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub archive_sha256: String,
}

/// 把 `src_dir` 下所有文件 (递归) 备份到 `dest_archive`。
pub async fn storage_sample_backup_export(
    src_dir: String,
    dest_archive: String,
) -> Result<StorageSampleBackupReceiptDto, StorageError> {
    let src = PathBuf::from(&src_dir);
    if !src.is_dir() {
        return Err(StorageError::not_found(format!(
            "src_dir not found: {src_dir}"
        )));
    }

    let mut entries = Vec::new();
    collect_files(&src, &src, &mut entries).await?;

    let mut total: u64 = 0;
    let mut body_lines = Vec::with_capacity(entries.len());
    let engine = base64::engine::general_purpose::STANDARD_NO_PAD;
    for rel in &entries {
        let abs = src.join(rel);
        let bytes = fs::read(&abs).await?;
        total += bytes.len() as u64;
        if total > MAX_TOTAL_BYTES {
            return Err(StorageError::quota_exceeded(format!(
                "archive exceeds {MAX_TOTAL_BYTES} bytes"
            )));
        }
        let sha = sha256_hex(&bytes);
        let b64 = engine.encode(&bytes);
        body_lines.push(format!(
            "{{\"rel\":{},\"size\":{},\"sha256\":\"{}\",\"b64\":\"{}\"}}",
            json_string(&rel.to_string_lossy()),
            bytes.len(),
            sha,
            b64
        ));
    }

    let body_concat = body_lines.join("\n");
    let archive_sha = sha256_hex(body_concat.as_bytes());
    let manifest = format!(
        "{{\"version\":{},\"created_ms\":{},\"file_count\":{},\"sha256\":\"{}\"}}",
        VERSION,
        now_ms(),
        entries.len(),
        archive_sha
    );

    let dest = PathBuf::from(&dest_archive);
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }
    let tmp = {
        let mut t = dest.clone();
        t.set_extension("archive.tmp");
        t
    };
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)
            .await?;
        f.write_all(manifest.as_bytes()).await?;
        f.write_all(b"\n").await?;
        f.write_all(body_concat.as_bytes()).await?;
        if !body_concat.is_empty() {
            f.write_all(b"\n").await?;
        }
        f.sync_all().await?;
    }
    fs::rename(&tmp, &dest).await?;

    Ok(StorageSampleBackupReceiptDto {
        archive_path: dest_archive,
        file_count: entries.len() as u64,
        total_bytes: total,
        archive_sha256: archive_sha,
    })
}

/// 从 archive 还原到 `dest_dir`。`overwrite=false` 时若目标文件存在则返回 `conflict`。
pub async fn storage_sample_backup_import(
    archive: String,
    dest_dir: String,
    overwrite: bool,
) -> Result<StorageSampleBackupReceiptDto, StorageError> {
    let file = fs::File::open(&archive)
        .await
        .map_err(|e| -> StorageError {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::not_found(archive.clone())
            } else {
                e.into()
            }
        })?;
    let mut reader = BufReader::new(file);
    let mut manifest_line = String::new();
    let n = reader.read_line(&mut manifest_line).await?;
    if n == 0 {
        return Err(StorageError::corrupted("archive empty"));
    }
    let trimmed = manifest_line.trim_end_matches(['\n', '\r']);
    let manifest = parse_manifest(trimmed)?;
    if manifest.version != VERSION {
        return Err(StorageError::invalid_argument(format!(
            "unsupported archive version: {}",
            manifest.version
        )));
    }

    let mut body_lines: Vec<String> = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let t = line.trim_end_matches(['\n', '\r']).to_string();
        if t.is_empty() {
            continue;
        }
        body_lines.push(t);
    }
    let body_concat = body_lines.join("\n");
    let actual_sha = sha256_hex(body_concat.as_bytes());
    if actual_sha != manifest.sha256 {
        return Err(StorageError::corrupted("archive sha256 mismatch"));
    }
    if body_lines.len() as u64 != manifest.file_count {
        return Err(StorageError::corrupted(format!(
            "file_count mismatch: manifest {}, actual {}",
            manifest.file_count,
            body_lines.len()
        )));
    }

    let dest = PathBuf::from(&dest_dir);
    fs::create_dir_all(&dest).await?;
    let engine = base64::engine::general_purpose::STANDARD_NO_PAD;
    let mut total: u64 = 0;
    for body in &body_lines {
        let item = parse_body_item(body)?;
        let bytes = engine
            .decode(item.b64.as_bytes())
            .map_err(|_| StorageError::corrupted("base64 decode failed"))?;
        if bytes.len() as u64 != item.size {
            return Err(StorageError::corrupted(format!(
                "size mismatch for {}",
                item.rel
            )));
        }
        let sha = sha256_hex(&bytes);
        if sha != item.sha256 {
            return Err(StorageError::corrupted(format!(
                "sha256 mismatch for {}",
                item.rel
            )));
        }
        validate_rel_path(&item.rel)?;
        let target = dest.join(&item.rel);
        if !overwrite && fs::metadata(&target).await.is_ok() {
            return Err(StorageError::conflict(format!(
                "target exists: {}",
                item.rel
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&target)
            .await?;
        f.write_all(&bytes).await?;
        f.sync_all().await?;
        total += bytes.len() as u64;
    }

    Ok(StorageSampleBackupReceiptDto {
        archive_path: archive,
        file_count: manifest.file_count,
        total_bytes: total,
        archive_sha256: manifest.sha256,
    })
}

struct Manifest {
    version: u32,
    sha256: String,
    file_count: u64,
}

struct BodyItem {
    rel: String,
    size: u64,
    sha256: String,
    b64: String,
}

fn parse_manifest(line: &str) -> Result<Manifest, StorageError> {
    let s = line.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err(StorageError::corrupted("manifest not a JSON object"));
    }
    let body = &s[1..s.len() - 1];
    Ok(Manifest {
        version: extract_number(body, "\"version\":")
            .ok_or_else(|| StorageError::corrupted("manifest.version missing"))? as u32,
        sha256: extract_string(body, "\"sha256\":")
            .ok_or_else(|| StorageError::corrupted("manifest.sha256 missing"))?,
        file_count: extract_number(body, "\"file_count\":")
            .ok_or_else(|| StorageError::corrupted("manifest.file_count missing"))?,
    })
}

fn parse_body_item(line: &str) -> Result<BodyItem, StorageError> {
    let s = line.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err(StorageError::corrupted("body line not a JSON object"));
    }
    let body = &s[1..s.len() - 1];
    Ok(BodyItem {
        rel: extract_string(body, "\"rel\":")
            .ok_or_else(|| StorageError::corrupted("rel missing"))?,
        size: extract_number(body, "\"size\":")
            .ok_or_else(|| StorageError::corrupted("size missing"))?,
        sha256: extract_string(body, "\"sha256\":")
            .ok_or_else(|| StorageError::corrupted("sha256 missing"))?,
        b64: extract_string(body, "\"b64\":")
            .ok_or_else(|| StorageError::corrupted("b64 missing"))?,
    })
}

async fn collect_files(
    base: &Path,
    cur: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), StorageError> {
    let mut rd = fs::read_dir(cur).await?;
    while let Some(entry) = rd.next_entry().await? {
        let meta = entry.metadata().await?;
        let p = entry.path();
        if meta.is_dir() {
            Box::pin(collect_files(base, &p, out)).await?;
        } else {
            let rel = p
                .strip_prefix(base)
                .map_err(|_| StorageError::internal("strip_prefix failed"))?
                .to_path_buf();
            out.push(rel);
        }
    }
    Ok(())
}

fn validate_rel_path(p: &str) -> Result<(), StorageError> {
    if p.is_empty() {
        return Err(StorageError::corrupted("rel must not be empty"));
    }
    if p.starts_with('/') || p.contains("..") {
        return Err(StorageError::corrupted(format!(
            "unsafe rel path: {p}"
        )));
    }
    Ok(())
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn extract_number(body: &str, key: &str) -> Option<u64> {
    let i = body.find(key)?;
    let after = &body[i + key.len()..];
    let end = after
        .find(|c: char| c == ',' || c == '}')
        .unwrap_or(after.len());
    after[..end].trim().parse().ok()
}

fn extract_string(body: &str, key: &str) -> Option<String> {
    let i = body.find(key)?;
    let after = &body[i + key.len()..];
    let after = after.trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
