//! 共享边界类型: `StorageError` + 常用小型 DTO。
//!
//! 与 `transport::common::TransportError` 同形 (扁平 struct + `code` 字符串常量),
//! 不引入带数据的 sum-type, 避开 FRB 的 freezed 依赖。
//!
//! 这里只放跨模式复用的"信号类型", 不放任何业务 DTO。

use flutter_rust_bridge::frb;

/// 跨边界的标准化存储错误。Dart 侧据 `code` 字段映射 UI 文案 / 重试策略。
///
/// `code` 取值范围见 [`StorageErrorCode`]。任何 Rust 内部错误 (含 `io::Error` /
/// `sled::Error` / `serde_json::Error`) 在抵达 FFI 边界前都应转成此结构,
/// 不暴露内部类型字符串。
#[derive(Debug, Clone)]
pub struct StorageError {
    pub code: String,
    pub message: String,
}

/// 错误码常量。Dart 侧 `error_contract.dart` 镜像。
#[frb(ignore)]
pub struct StorageErrorCode {}

#[allow(non_upper_case_globals)]
impl StorageErrorCode {
    /// 调用方参数非法 (空 key、超长 value、非法路径等)。
    pub const InvalidArgument: &'static str = "invalid_argument";
    /// 目标不存在 (key 未写入、文件已被删除、快照 id 未知等)。
    pub const NotFound: &'static str = "not_found";
    /// 期望状态被并发改写 (乐观锁失败、重复初始化、tombstone 冲突)。
    pub const Conflict: &'static str = "conflict";
    /// 持久化数据校验失败 (sha256 不匹配、json 解析失败、密文 tag 错误)。
    pub const Corrupted: &'static str = "corrupted";
    /// 资源限额: 超出最大 key/value 数量、磁盘配额、缓存容量上限。
    pub const QuotaExceeded: &'static str = "quota_exceeded";
    /// 内部故障 (io::Error、sled 故障); 不暴露细节。
    pub const Internal: &'static str = "internal";
}

impl StorageError {
    #[frb(ignore)]
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::InvalidArgument.into(),
            message: msg.into(),
        }
    }
    #[frb(ignore)]
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::NotFound.into(),
            message: msg.into(),
        }
    }
    #[frb(ignore)]
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::Conflict.into(),
            message: msg.into(),
        }
    }
    #[frb(ignore)]
    pub fn corrupted(msg: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::Corrupted.into(),
            message: msg.into(),
        }
    }
    #[frb(ignore)]
    pub fn quota_exceeded(msg: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::QuotaExceeded.into(),
            message: msg.into(),
        }
    }
    #[frb(ignore)]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: StorageErrorCode::Internal.into(),
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => StorageError::not_found(e.to_string()),
            std::io::ErrorKind::AlreadyExists => StorageError::conflict(e.to_string()),
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => {
                StorageError::invalid_argument(e.to_string())
            }
            _ => StorageError::internal(e.to_string()),
        }
    }
}

/// 通用回执: 内容寻址 / sha256 校验场景的统一返回 (模式 C/J 共用)。
#[derive(Debug, Clone)]
pub struct StorageSampleBlobReceiptDto {
    /// 写入后 / 校验后的资源 ID (一般是 hex sha256 前缀)。
    pub id: String,
    /// 总字节数。
    pub size_bytes: u64,
    /// 内容 sha256 (64 hex)。
    pub sha256_hex: String,
}

/// 给定字节计算 sha256 (lowercase hex)。多个 pattern 复用。
#[frb(ignore)]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

/// 当前 wall-clock 毫秒。仅用于"用户可见时间戳"; **不要**用于 deadline / 超时计时。
#[frb(ignore)]
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
