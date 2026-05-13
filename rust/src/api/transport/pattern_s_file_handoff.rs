//! 场景关键词: 数百 MB / 大文件 / 带外传输 / 视频 / 数据库导入 → 选我。
//!
//! 模式 S: 文件路径握手。
//!
//! 设计:
//! - Dart 写文件到双方约定的临时目录 (e.g. `path_provider.getTemporaryDirectory()`)。
//! - Dart 调用 Rust, 只传**路径字符串** + 可选的预期 `sha256` 校验码。
//! - Rust 流式读取并 (可选) 校验内容; 零拷贝, 不经过 FRB 的序列化层。
//! - 责任归属: 默认 Rust 在成功后删除临时文件 (受 `delete_after` 控制)。
//!
//! 为什么不用模式 G:
//! - >10MB 时, `Vec<u8>` 仍要把数据从 Dart 堆复制到原生堆 (FRB 序列化代价)。
//! - 路径握手则纯靠文件系统页缓存, 几乎零额外内存。

use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use super::common::TransportError;

/// 文件处理回执。
///
/// `verified` 只在调用方提供了 `expected_sha256` 时有意义:
/// - `Some(true)`: 内容哈希与预期一致。
/// - `Some(false)`: 不一致 (此时函数会返回 `Err(conflict)`, 调用方应不会看到此值)。
/// - `None`: 未要求校验。
#[derive(Debug, Clone)]
pub struct TransportSampleFileReceiptDto {
    pub bytes_read: u64,
    pub sha256_hex: String,
    pub verified: Option<bool>,
    pub deleted: bool,
}

/// 消费一个 Dart 写到磁盘上的文件。
///
/// - `expected_sha256`: 若提供, 与实际内容比对失败时返回 `conflict`。
/// - `delete_after`: 处理成功后是否删除原文件。删除失败不视为错误 (回执的 `deleted` 字段会为 false)。
pub async fn transport_sample_consume_file(
    path: String,
    expected_sha256: Option<String>,
    delete_after: bool,
) -> Result<TransportSampleFileReceiptDto, TransportError> {
    let p = Path::new(&path);
    let meta = tokio::fs::metadata(p).await.map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => TransportError::not_found(format!("file not found: {path}")),
        std::io::ErrorKind::PermissionDenied => {
            TransportError::invalid_argument(format!("permission denied: {path}"))
        }
        _ => TransportError::internal(format!("stat failed: {e}")),
    })?;
    if !meta.is_file() {
        return Err(TransportError::invalid_argument(
            "path is not a regular file",
        ));
    }

    let mut file = tokio::fs::File::open(p)
        .await
        .map_err(|e| TransportError::internal(format!("open failed: {e}")))?;

    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = file
            .read(&mut buf)
            .await
            .map_err(|e| TransportError::internal(format!("read failed: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total = total.saturating_add(n as u64);
    }
    let sha256_hex = hex_lower(&hasher.finalize());

    let verified = match expected_sha256.as_deref() {
        None => None,
        Some(exp) => {
            if !exp.eq_ignore_ascii_case(&sha256_hex) {
                return Err(TransportError::conflict(
                    "sha256 mismatch between dart-side and rust-side read",
                ));
            }
            Some(true)
        }
    };

    let deleted = if delete_after {
        tokio::fs::remove_file(p).await.is_ok()
    } else {
        false
    };

    Ok(TransportSampleFileReceiptDto {
        bytes_read: total,
        sha256_hex,
        verified,
        deleted,
    })
}

#[inline]
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}
