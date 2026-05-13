# `transport`: Flutter ↔ Rust 数据传输模式标本库

> 一句话: **"我想把 X 从 Dart 送给 Rust（或反向），用哪种姿势？"** ——
> 这个库的每个 `pattern_*` 都是一种姿势的最小可运行参考。

## 为什么有这个库

工程手册 §6.13 已经把 Flutter↔Rust 调用模式系统地归纳为 17 个 + 速查表。本库把它们变成可直接拷贝的代码，并补齐了 R/S 两个边界情况和 T/U 两个跨边界硬约束（共 20 个模式 + 7 个 Dart 助手）：

- Rust 侧: `rust/src/api/transport/pattern_*.rs`（FRB 自动生成 Dart 绑定）
- Dart 侧: `lib/transport/*.dart`（J/L/N/O/I/Q 等纯 Dart 模式 + 错误契约）
- 统一入口: `import 'package:fragments/transport/transport.dart';`

每个模式独立、零业务耦合，方便：

1. **新人查阅**: "我要做 X" → 决策树 → 锁定模式编号 → 打开对应文件。
2. **代码复用**: 拷一个 `pattern_*` 到新项目, 改 DTO 即可用。
3. **架构审查**: 评审 PR 时, 反问"你用的是模式几"。

## 决策树入口

详见 [`decision-tree.md`](./decision-tree.md)。速查节选：

| 我要做 | 模式 | 入口文件 |
|---|---|---|
| 同步纯计算（<100us, 无 IO） | A | `pattern_a_sync.rs` |
| 一次性异步业务用例 | B | `pattern_b_async.rs` |
| 持续接收事件 | C | `pattern_c_stream.rs` |
| 用户可中途取消 | D | `pattern_d_cancel.rs` |
| 进度条 + 取消 | E | `pattern_e_progress.rs` |
| 跨多次调用持有状态 | F | `pattern_f_opaque.rs` |
| 大二进制 / 分块 | G | `pattern_g_bytes.rs` |
| 双向 REPL | H | `pattern_h_duplex.rs` |
| 全局事件总线 | I | `pattern_i_event_bus.rs` + `event_bus.dart` |
| 短时请求合并 | J | `coalescing.dart` |
| 分页 | K | `pattern_k_pagination.rs` |
| 重试 / 幂等 | L | `retry.dart` + `pattern_l_idempotent.rs` |
| Dart 注入回调 | M | `pattern_m_callback.rs` |
| 并发限流 | N | `pool.dart` + `pattern_n_semaphore.rs` |
| 后台 isolate 跑 Rust | O | `isolate.dart` |
| 启动初始化 / 单例 | P | `pattern_p_singleton.rs` |
| Mock / 测试 | Q | `pattern_q_mock.rs` + `mock.dart` |
| 动态 schema / JSON | R | `pattern_r_json.rs` |
| 大文件路径握手 | S | `pattern_s_file_handoff.rs` |
| panic 安全 / 防 FFI 崩溃 | T | `pattern_t_panic_safety.rs` |
| 结构化并发 / fan-out / race | U | `pattern_u_structured.rs` |
| 请求横切元数据 (request_id / deadline / 幂等 / trace) | V | `pattern_v_request_meta.rs` + `request_context.dart` |

## 错误契约

所有 Rust 入口返回 `Result<T, TransportError>`。`TransportError` 是扁平 struct（不是 Rust enum-with-data，避免触发 FRB freezed 依赖）:

```rust
TransportError { code: String, message: String, elapsed_ms: u64 }
```

`code` 字段取值常量见 `common.rs::TransportErrorCode`; Dart 镜像在 `error_contract.dart::TransportErrorCodes`。完整规则参见 [`error-contract.md`](./error-contract.md)。

## 通用陷阱

参见 [`pitfalls.md`](./pitfalls.md)（与手册 §6.13.18 摘录互为补充）。

## 数据形状决策

如何为新模式挑选 DTO 形状 (强类型 / JSON / 字节 / 元数据) 参见 [`data-shapes.md`](./data-shapes.md)。

## 设计原则（库的"宪法"）

1. **零业务耦合**: 不引用 `crate::domain` / `application` / `fragment_repository`。
2. **样本前缀**: 所有示例 DTO 用 `TransportSample*` 前缀, 提醒"标本不可生产化复用名字"。
3. **每个 pattern 单文件**: 拷贝 1 个 `.rs` + `common.rs` 即可在新项目独立工作。
4. **不引入隐藏依赖**: 仅显式添加 `tokio`（带 `sync`/`time`/`fs`/`rt`/`macros`/`io-util` feature）、`futures`、`sha2`。
5. **panic 不穿 FFI**: 任何 `tokio::spawn` 出去的逻辑都该过模式 T 的 `catch_panic_async`。
6. **并发要结构化**: 多子任务用模式 U 的 `join_all`/`select_all`, 不要散养 `spawn`。

## 我要新加一个模式

1. 在 `rust/src/api/transport/` 新建 `pattern_x_xxx.rs`，文件首行 `//!` 写"场景关键词 → 选我"。
2. 在 `mod.rs` 中 `pub mod pattern_x_xxx;`。
3. 跑 `flutter_rust_bridge_codegen generate`。
4. 在 `transport.dart` barrel 加 export。
5. 在本文件决策树表里加一行。
6. 在 `test/transport/` 加 happy-path 测试。

## 重新代码生成

```bash
flutter_rust_bridge_codegen generate
```

新增 / 删除 / 改签名的 Rust 入口都需要重跑。生成结果写入 `lib/src/rust/api/transport/`。
