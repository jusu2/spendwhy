# Transport 错误契约

与工程手册 §11.2 一致。本库强制所有 Rust 入口的错误路径统一收敛为 `TransportError`, 禁止把 `anyhow::Error` 原文跨边界透传。

## 类型定义

Rust 侧 (`rust/src/api/transport/common.rs`):

```rust
pub struct TransportError {
    pub code: String,        // 见下表
    pub message: String,     // 不含 PII / 堆栈
    pub elapsed_ms: u64,     // 仅 timeout 用
}
```

Dart 侧由 FRB 自动生成对等类; 加上 `error_contract.dart` 提供的便利访问器。

## 错误码

| code | 语义 | HTTP 类比 | Dart 可重试? |
|---|---|---|---|
| `invalid_argument` | 调用方参数非法 | 400 | 否 |
| `not_found` | 资源不存在 | 404 | 否 |
| `conflict` | 与现有状态冲突 (唯一键 / 乐观锁 / 会话已 dispose) | 409 | 否 |
| `canceled` | 用户主动取消 / `CancelHandle.cancel()` | 499 | 否 |
| `timeout` | 操作超时 (`elapsed_ms` 填写) | 504 | 是 |
| `internal` | 其余不可恢复内部错误 | 500 | 是 |

`error_contract.dart::TransportErrorX` 扩展提供 `isCanceled` / `isTimeout` / `isRetriable` 等便利判定。

## 业务侧使用

```dart
try {
  final v = await transportSampleCompute(input: 'hello');
} on TransportError catch (e) {
  if (e.isInvalidArgument) {
    // 给用户文案: 校验失败
  } else if (e.isRetriable) {
    // 走 retry.dart
  } else {
    // 上报 / fail-fast
  }
}
```

## 不要这样做

- ❌ Rust 侧 `return Err(anyhow!("..."))` 直接抛过边界 → 包成 `TransportError::internal(...)`。
- ❌ 在 `message` 里塞用户输入或敏感信息 → 用 `code` 走文案路由。
- ❌ 把成功路径上的"特殊值"塞进 `message` (例如把 ID 当 message 回传) → 用专门 DTO。
- ❌ 在 Dart 侧把 `TransportError` rethrow 时再裹一层异常 → 让原 `code` 一路冒泡到 UI 层 / 上报点。

## 与业务错误的关系

本库 **不** 复用 `crate::api::error::AppError` 或 `crate::error::AppResult`。业务代码可以在自己的服务层把 `TransportError` 翻译成业务错误：

```dart
final result = await transportSampleCompute(input: x)
    .catchError((e) => throw BusinessError.from(e));
```

理由: 模式库是"骨架样本", 业务错误是"血肉", 二者不该在同一 enum 里。
