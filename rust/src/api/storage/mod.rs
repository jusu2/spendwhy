//! Flutter ↔ Rust 数据**存储**模式标本库。
//!
//! 与 [`crate::api::transport`] 平行: transport 解决"如何通信", storage 解决
//! "如何把 X 存到哪 / 怎么存 / 重启后还想要 / 加密 / 备份 / 迁移"。
//!
//! 设计原则:
//! - 零业务耦合: 不引用 `crate::domain` / `crate::application` / `lib/data/`。
//! - `StorageSample*` 前缀: 强提醒"标本不可生产化复用名字"。
//! - 每个 `pattern_*` 单文件可拷贝, 仅依赖 [`common`] + 一个核心 crate。
//! - 路径都通过参数注入, 不在 Rust 侧调用 `path_provider`。
//! - fsync 默认开启, 显式参数控制可禁用。
//! - 加密永远是 envelope: 数据密钥 ≠ 主密钥, 主密钥由 Dart 端 SecureStorage 提供。
//!
//! 选择哪个模式? 看 `docs/storage/decision-tree.md` 或下表:
//! - 内存 TTL+LRU 缓存            → [`pattern_a_memory`]
//! - 原子文件写 (断电安全)         → [`pattern_b_atomic_file`]
//! - Blob 内容寻址                → [`pattern_c_blob`]
//! - 版本化快照 (last-N)          → [`pattern_d_snapshot`]
//! - 追加事件日志 (NDJSON)        → [`pattern_e_event_log`]
//! - 有序 KV + range scan         → [`pattern_f_ordered_kv`]
//! - 设置 KV (小、明文、常读)     → [`pattern_g_settings`]
//! - 持久 TTL+LRU 缓存            → [`pattern_h_persistent_cache`]
//! - 字段级信封加密 (AES-GCM)     → [`pattern_i_encryption`]
//! - 备份 / 导出 / 导入           → [`pattern_j_backup`]
//!
//! Dart 侧 (K..R) 在 `lib/storage/`: secure_storage / sql / migration / outbox /
//! tenant / backfill / soft_delete / cache_combinator。

pub mod common;

pub mod pattern_a_memory;
pub mod pattern_b_atomic_file;
pub mod pattern_c_blob;
pub mod pattern_d_snapshot;
pub mod pattern_e_event_log;
pub mod pattern_f_ordered_kv;
pub mod pattern_g_settings;
pub mod pattern_h_persistent_cache;
pub mod pattern_i_encryption;
pub mod pattern_j_backup;
