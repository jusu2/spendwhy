# FFI Error Contract

`archforge-ffi` 与跨 ABI 调用方之间的稳定数据约定。任何想接 archforge 的语言
(Dart / Swift / Kotlin / C# / TS) 都按这份文档实现自己的反序列化层。

## 1. 跨边界形状

```
{
  "kind":     "<snake_case discriminator>",
  "message":  "<human readable, already redacted>",
  "is_panic": <bool, default false>
}
```

- `kind` 是字符串枚举, 见 §2。字段顺序在 JSON 里不强求, 但**字段名**是契约。
- `message` 已经是给用户/操作员看的文本; **不应再加堆栈/内部状态**。
- `is_panic` 缺省视为 `false`(兼容老版本 Rust producer)。

## 2. `kind` 取值

| 值 | 触发场景 | 客户端典型行为 |
|---|---|---|
| `not_found` | 资源不存在 (用户/记录/Key) | 提示 "找不到", 不重试 |
| `conflict` | 重复键 / 乐观锁冲突 / 状态冲突 | 提示用户重做; 不要重试同一请求 |
| `invalid` | 入参违反域不变量 (e.g. 邮箱格式) | 把 `message` 显示在该输入框下方 |
| `unavailable` | 外部依赖暂时不可用 | **可重试**, 指数退避 |
| `forbidden` | 调用者没权限 | 跳登录或权限申请页 |
| `deadline_exceeded` | `Context.deadline` 到期 | 提示 "超时", 视业务可重试 |
| `internal` | 不可恢复 / 真 bug / 被 panic | 配合 `is_panic` 决定上报路径 |
| `unknown` | 客户端看到了不认识的 `kind` | **必须有这个分支**, 当 `internal` 处理 |

> **向前兼容**: Rust 端新增的 variant 会以新 `kind` 字符串到达旧客户端。旧客户端的 `unknown` 分支兜底, 不会崩。

## 3. `is_panic` 的语义

- `true`: Rust 端 `archforge-ffi::guard_*` 捕获了一个 panic。这是 ABI 边界的"我们刚救了一条命"信号。
  - 客户端应该走**崩溃上报通道**(Sentry / Crashlytics / Firebase Crash) 而不是当业务错误。
  - 用户提示建议统一为"程序内部错误, 已上报"。
- `false`: 业务流程主动产生的错误。客户端按 `kind` 路由。

实现细节: 当 `AppError::Internal(msg)` 的 `msg` 以 `"panic: "` 开头时, Rust 自动置 `is_panic = true`。
这个 prefix 是稳定契约 (`archforge_ffi::PANIC_INTERNAL_TAG`)。

## 4. 单向性 — 不要在 Rust 端反序列化为 `AppError`

`archforge_kernel::AppError` 故意**不实现** `Deserialize`。这条线必须守住:

- ✅ Rust → Dart: `AppError → WireError → JSON`
- ✅ Dart 内部解析 JSON → Dart `AppException` 镜像类
- ❌ Dart → JSON → 反向回 Rust 的 `AppError`
- ❌ 任何把 `WireError` 转回 `AppError` 的 helper

为什么? 见 `archforge/ARCHITECTURE_INVARIANTS.md` §4: 错误枚举可被反序列化 ⇒ 上游 match arm 可被攻击者伪造命中。
跨进程 / 跨节点的"错误传播"需要 contract 层另外定义 `contract-*::WireRequestError`, 不复用 kernel `AppError`。

## 5. Dart 端实现约定

镜像类放在 `lib/archforge/wire_error.dart`, 与本文档逐字段对应:

```dart
enum WireErrorKind {
  notFound, conflict, invalid, unavailable,
  forbidden, deadlineExceeded, internal, unknown,
}

@immutable
class WireError implements Exception {
  final WireErrorKind kind;
  final String message;
  final bool isPanic;
  // 解析时遇到陌生 kind → unknown (永不抛)
  factory WireError.fromJson(Map<String, dynamic> json) {...}
}
```

- 解析必须**永不抛**。陌生 `kind` 走 `unknown`; 字段缺失走默认值。
- `isRetriable` getter 建议: `kind == unavailable || kind == deadlineExceeded`。
- `isPanic == true` 时, 触发应用自己的崩溃上报器。

## 6. 版本演进规则

| 改动 | 兼容性 | 怎么发版 |
|---|---|---|
| 新增 `kind` 字符串 (Rust 端) | 向后兼容 (旧客户端 → `unknown`) | minor |
| 重命名既有 `kind` 字符串 | **破坏** | major + ADR |
| 新增 `WireError` 顶层字段 | 兼容(默认值) | minor |
| 删除 `WireError` 字段 | **破坏** | major + ADR |
| 改 `message` 文案 | 兼容(message 不应被客户端 match) | patch |
| 改 `is_panic` 触发条件 | 兼容(语义保持: panic ⇒ true) | minor |

## 7. 测试该测什么

Rust 端:
- 每个 `AppError` variant 映射到正确 `WireErrorKind`。
- `AppError::Internal("panic: ...")` 置 `is_panic = true`。
- 普通 `AppError::Internal("...")` 不置 `is_panic`。
- JSON 形状 (字段名、`snake_case` 枚举) 锁定。

Dart 端:
- 每个 `WireErrorKind` 解析正确。
- 陌生 `kind` → `unknown`, 不抛。
- `is_panic` 缺失 → `false`。
- `WireException` 的 `toString()` 包含 `kind` 与 (若 panic) 标记, 便于日志。
