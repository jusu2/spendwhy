# FFI Patterns

跨语言边界(Rust ↔ Dart / C / Swift / Kotlin)的稳定基础设施。

## 文档

- [`decision-tree.md`](./decision-tree.md) — 何时用哪种 guard、和现有 transport pattern T 的关系。
- [`error-contract.md`](./error-contract.md) — 错误 DTO 的稳定 JSON 形状与版本演进规则。

## 实现

- Rust 侧: [`archforge/ffi/`](../../archforge/ffi/)
  - `guard_sync` / `guard_async` — panic 隔离守门员
  - `WireError` — wire-safe 错误 DTO (单向 `From<AppError>`)
  - `PanicReporter` — 全局 panic 上报钩子
- Dart 侧: [`lib/archforge/wire_error.dart`](../../lib/archforge/wire_error.dart)
  - `WireError` / `WireErrorKind` / `WireException`
  - JSON 解析永不抛, 陌生 `kind` → `unknown`

## 与既有 transport `pattern_t_panic_safety` 的关系

`pattern_t_panic_safety` (在 `rust/src/api/transport/`) 把 panic 转成 `TransportError`,
适合 transport 切片内部的演示与教学。

`archforge_ffi` 把 panic 转成 `archforge_kernel::AppError`, 适合**跨切片**的生产代码:
auth、billing、sync、任何 use case 都用同一对 guard。

新业务请直接用 `archforge_ffi::guard_*`; `pattern_t` 保留为教学样例不删除。
