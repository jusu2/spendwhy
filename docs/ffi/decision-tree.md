# FFI 决策树

何时用 `archforge-ffi`、用哪一对 API、和现有 transport `pattern_t` 的关系。
按问题顺序往下走。

## Step 1: 你在写什么?

- **Rust 库函数, 公开给 Dart / C / Swift / Kotlin (任何 ABI 外的调用方)**
  → 一定要走 ffi。继续 Step 2。
- **Rust 库内部 helper, 不跨 ABI**
  → 不需要。`Result<T, AppError>` 自然回传, panic 由调用栈最上面那个 ffi 守门员兜住。
- **测试代码 / `examples/*` 二进制**
  → 不需要。panic 直接崩进程, 是测试该有的行为。

## Step 2: 同步还是异步?

- 同步函数 (无 `.await`)               → `guard_sync(|| use_case(...))`
- 异步函数 (含 `.await`, 跨 `tokio::spawn`) → `guard_async(async move { ... })`

`guard_async` 必须包住 **整个** future, 而不仅仅是 `.await` 一行 ——
panic 可能出现在第一行同步代码里, `AssertUnwindSafe(fut)` 才能兜住。

## Step 3: 调用方法 (返回值类型)

- 调用方还在 Rust (例: `bridge-frb` 用 `?` 解开)
  → 返回 `Result<T, AppError>`。让 FRB 的自动派生处理 `kind/detail`。
- 调用方在 ABI 外 (Dart / C, 你自己手写胶水)
  → 用 `WireError::from_result(guarded)` 把 `AppError` 转成可 `Deserialize` 的 DTO。
  原因: `AppError` 是 `Serialize`-only, 不能从 wire 反序列化(见 `archforge-kernel` 不变量 #4)。

## Step 4: 我是不是应该在每一层都加 guard?

**不**。**每个 ABI 入口点 1 个 guard**, 足够。

- 嵌套 guard 是幂等的(内层先把 panic 转成 `AppError::Internal`, 外层看到的就只是 `Err`),
  但会让日志重复, 也容易掩盖真正的 panic 位置。
- 唯一例外: `tokio::spawn(async move { ... })` 内部 **必须** 自己包 `guard_async` ——
  spawn 出去的 future 不在 ABI 调用栈上, 它的 panic 不会被入口的 guard 捕到。
  见 [`rust/src/api/transport/pattern_t_panic_safety.rs`](../../rust/src/api/transport/pattern_t_panic_safety.rs)。

## Step 5: 我要把 panic 报到哪儿?

- **想要 tracing / Sentry / Crashlytics 上报**
  → 实现 `PanicReporter`, 在进程启动时 `install_panic_reporter(MyReporter)`。
- **不想报**
  → 什么都不做。默认无 reporter, panic 仍被捕、仍转成 `AppError::Internal`,
  只是不进日志。这对短命 CLI / 测试 OK; 对长跑服务/桌面 App **必装**。
- **想要"按错误种类"路由**
  → 在 reporter 里看 `event.site` (`"guard_sync"` / `"guard_async"`)。
  对 `is_panic = true` 的 `WireError` 走 crash 流, 其余走 ops 流。

## Step 6: panic 是真 panic, 还是我用 `panic!` 当快速返回?

**永远不要这么做**。`panic!` 在 ffi crate 边界等于一次"我们刚刚救了一条命"的告警。
快速返回用 `return Err(AppError::Invalid("..."))`。
保留 `panic!` 给真正不可恢复的 bug ("我的代码不可能走到这里, 走到了就是有 bug")。

---

## 和 `pattern_t_panic_safety` 的关系

| 维度 | `archforge-ffi::guard_*` | `rust/src/api/transport/pattern_t_panic_safety::catch_panic_*` |
|---|---|---|
| 错误类型 | `archforge_kernel::AppError` (跨切片通用) | `TransportError` (transport 切片专用) |
| 所在层 | 跨域 kernel 之上, 任何 slice 都能用 | 仅 rust/ transport 切片内部, 演示意义大 |
| 何时新建 use case | **默认走这里** | 仅当 use case 真的只在 transport 演示库里跑 |
| 与 Reporter 集成 | 内建 `PanicReporter` 全局钩子 | 无, 注释里建议 `tracing::error!` |

迁移建议: 新代码统一用 `archforge_ffi::guard_*`; `pattern_t` 保留为"transport 边界自助 demo"的教学样例。

---

## 常见场景 → 选择

| 场景 | 选择 |
|---|---|
| FRB 暴露的 `pub async fn ffi_create_user(...)` | `guard_async` + `WireError::from_result` |
| FRB 暴露的 `pub fn ffi_decode_token(...)` (同步) | `guard_sync` + `WireError::from_result` |
| `tokio::spawn` 出去的后台任务 | `guard_async` 包整个 future |
| 库 helper, 只被其他 Rust 调用 | 不要 guard, 自然 `Result` 返回 |
| 测试用 mock 函数, 想看到真正 panic 栈 | 不要 guard |
| 多个 ffi 入口共用一个底层 use case | guard 在 ffi 边界, 不在底层 |
| 同进程内还要把错误持久化进数据库 | 用 `AppError` (Serialize-only) 落库; 不要把 `WireError` 当数据 |

## 不变量 (违反 = PR 拒)

1. **没有 `Deserialize` 路径回到 `AppError`**。手写胶水也不行。如果你需要把 wire JSON 在 Rust 内重新提升为业务错误, 那是 contract 层的事, 加一个 `contract-*::WireRequestError` DTO, 不要碰 kernel。
2. **`guard_*` 包的闭包/future 必须 `AssertUnwindSafe` 语义安全**。如果你在闭包里持锁、写到一半文件, panic 会留下半成品状态。把 `Mutex` 换 `parking_lot::Mutex`, 或在 `Drop` 里显式回滚。
3. **panic 的细节不直接回给前端**。`WireError.message` 会带上 `"panic: ..."`, 这本身没问题, 但**不要再加 stack trace、内部变量值**。生产期记到 reporter 即可。
4. **每个 ABI 入口最多一层 guard**。嵌套是 noop, 但写出来就是噪音。
