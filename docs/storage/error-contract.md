# Storage 错误契约

与 `transport` 的错误模型同形 (扁平 struct + 字符串 code), 但语义针对持久化层调整。

## 类型定义

Rust 侧 (`rust/src/api/storage/common.rs`):

```rust
pub struct StorageError {
    pub code: String,    // 见下表
    pub message: String, // 不含 PII / 堆栈
}
```

Dart 侧由 FRB 自动生成对等类; 加上 `error_contract.dart` 提供的便利访问器。

## 错误码

| code | 语义 | 触发例 | Dart 可重试? |
|---|---|---|---|
| `invalid_argument` | 调用方参数非法 | 路径含 `..`, key 太长, ID 非 hex | 否 |
| `not_found` | 资源不存在 | 读不存在的 key / blob / snapshot | 否 |
| `conflict` | 与现有状态冲突 | atomic_write_if_absent 已存在, sha256 不匹配 | 否 |
| `corrupted` | 数据损坏 / 解密失败 / 校验失败 | AES tag 错, NDJSON 行畸形, 长度短于 header | 否 (要人工或恢复备份) |
| `quota_exceeded` | 超出容量 / 大小限制 | settings 满 4096 条, value >4KB | 否 |
| `internal` | 其余不可恢复 IO 错误 | 磁盘满 (部分), 权限错 | 是 (可能临时) |

`error_contract.dart::StorageErrorX` 扩展提供 `isNotFound` / `isConflict` / `isCorrupted`
/ `isRetriable` 等便利判定。

## 业务侧使用

```dart
try {
  final bytes = await storageSampleAtomicRead(path: p);
} on StorageError catch (e) {
  if (e.isNotFound) {
    // 首次启动, 走 init
  } else if (e.isCorrupted) {
    // 恢复备份 / 告警
  } else if (e.isRetriable) {
    // 短暂等待后重试
  } else {
    rethrow;
  }
}
```

## 不要这样做

- ❌ Rust 侧 `return Err(anyhow!("..."))` 直接抛过边界 → 包成 `StorageError::internal(...)`。
- ❌ 在 `message` 里塞绝对路径 / 用户文本 / 密钥 → 用 `code` 走文案路由, 仅记录在 tracing 日志。
- ❌ 把 "key 不存在" 当 `internal` 而非 `not_found` → 调用方无法区分"该重试"还是"该 init"。
- ❌ Dart 侧捕获 `StorageError` 后再裹一层异常 → 让原 `code` 一路冒泡到 UI 层 / 上报点。

## 与业务错误的关系

本库 **不** 复用 `crate::api::error::AppError` 或 `crate::error::AppResult`。
理由与 `transport` 同: 模式库是"骨架样本", 业务错误是"血肉", 二者不该在同一 enum 里。
业务代码可以在自己的服务层把 `StorageError` 翻译成业务错误:

```dart
final result = await storageSampleAtomicRead(path: p)
    .catchError((e) => throw RepositoryError.from(e));
```

## 为什么没有 `canceled` / `timeout`?

存储操作通常是短促的 IO; 没有 `transport` 里"长任务可取消"的场景。如果将来加
back-pressure / 长 backfill 入口, 再扩 `StorageErrorCode` 即可 (字段保持兼容,
新 code 只影响新接口)。
