# `storage`: 本地数据持久化模式标本库

> 一句话: **"我想把 X 存到哪里, 怎么存, 重启后还想要, 加密 / 备份 / 迁移 / 软删?"** ——
> 这个库的每个 `pattern_*` 是一种存储姿势的最小可运行参考。

## 与 `transport/` 的关系

`transport` 解决"Flutter↔Rust 通信"; `storage` 解决"Flutter / Rust 各自把数据持久化"。
两者**并列、不互相依赖**, 都遵循同样的标本馆规则: 零业务耦合 + `*Sample*` 前缀 +
每模式独立可拷贝。

## 与 `lib/data/` 的关系

`lib/data/` 是 **SpendWhy 业务持久化层** (FragmentRepository + AppDatabase + 现行
sqflite schema)。`lib/storage/` 是**通用工具集**, 不替换、不引用、不修改
`lib/data/` 任何一行; 它给 ADR-0002 / 0004 / 0005 规划的 Rust 持久化层做技术
储备, 同时给 Dart 端补一组与业务解耦的通用模式。

## 为什么有这个库

- Rust 侧: `rust/src/api/storage/pattern_*.rs` (FRB 自动生成 Dart 绑定)
- Dart 侧: `lib/storage/*.dart` (K..R 共 8 个纯 Dart 模式 + 错误契约)
- 统一入口: `import 'package:fragments/storage/storage.dart';`

## 决策树入口

详见 [`decision-tree.md`](./decision-tree.md)。速查节选:

| 我要做 | 模式 | 入口文件 |
|---|---|---|
| 进程内 TTL+LRU 内存缓存 | A | `pattern_a_memory.rs` |
| 原子文件写 (write-temp + fsync + rename) | B | `pattern_b_atomic_file.rs` |
| 大对象 / 内容寻址 Blob | C | `pattern_c_blob.rs` |
| 版本化快照 / undo / 回滚点 | D | `pattern_d_snapshot.rs` |
| 追加式事件日志 (NDJSON) | E | `pattern_e_event_log.rs` |
| 有序 KV + 前缀 / 范围扫描 | F | `pattern_f_ordered_kv.rs` |
| 设置 / 偏好 (小、明文、常读) | G | `pattern_g_settings.rs` |
| 重启保留的 TTL+LRU 缓存 | H | `pattern_h_persistent_cache.rs` |
| 字段级信封加密 (AES-256-GCM) | I | `pattern_i_encryption.rs` |
| 备份 / 导出 / 导入 | J | `pattern_j_backup.rs` |
| 系统安全存储 (Token / 主密钥) | K | `secure_storage.dart` |
| SQL 事务 + savepoint | L | `sql.dart` |
| Schema 迁移 (up/down) | M | `migration.dart` |
| Outbox / 离线写队列 | N | `outbox.dart` |
| 多租户 namespace 分区 | O | `tenant.dart` |
| 幂等批量 backfill | P | `backfill.dart` |
| 软删 + tombstone GC | Q | `soft_delete.dart` |
| Read-through / Write-back 缓存 | R | `cache_combinator.dart` |

## 错误契约

所有 Rust 入口返回 `Result<T, StorageError>`。`StorageError` 是扁平 struct
(与 `TransportError` 同形):

```rust
StorageError { code: String, message: String }
```

`code` 取值常量见 `common.rs::StorageErrorCode`; Dart 镜像在
`error_contract.dart::StorageErrorCodes`。完整规则参见
[`error-contract.md`](./error-contract.md)。

## 通用陷阱

参见 [`pitfalls.md`](./pitfalls.md)。

## 数据形状决策

如何按引擎 / 持久度 / 形状轴选模式参见 [`data-shapes.md`](./data-shapes.md)。

## 设计原则 (库的"宪法")

1. **零业务耦合**: 不引用 `crate::domain` / `application` / `lib/data/`。
2. **样本前缀**: 所有示例 DTO 用 `StorageSample*` 前缀。
3. **每个 pattern 单文件**: 拷贝 1 个 `.rs` + `common.rs` 即可在新项目独立工作。
4. **路径由调用方注入**: Rust 不私下解析 `path_provider`, 测试 / 生产分离干净。
5. **fsync 默认开启, 可显式禁用**: 默认走"安全慢", 性能优化要参数显式选。
6. **加密永远 envelope 模式**: 数据密钥 ≠ 主密钥, 主密钥从 Dart 侧
   `flutter_secure_storage` (模式 K) 提供。
7. **`StorageError` 同源, 不复用业务错误**: 与 `AppError` / `AppResult` 严格隔离。

## 重新代码生成

```bash
flutter_rust_bridge_codegen generate
```

新增 / 删除 / 改签名的 Rust 入口都需要重跑。生成结果写入
`lib/src/rust/api/storage/`。
