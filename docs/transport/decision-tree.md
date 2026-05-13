# Transport 决策树

按问题顺序往下走, 锁定模式编号。

## Step 1: 调用方向?

- Rust → Dart（Rust 主动推送） → Step 2A
- Dart → Rust（Dart 发起调用） → Step 2B
- 双向（命令 + 事件） → **模式 H** (`pattern_h_duplex`)

## Step 2A: Rust 主动推送

- 单次订阅, 流随函数返回 → **模式 C** (`pattern_c_stream`)
- 多订阅者 / 进程级常驻 → **模式 I** (`pattern_i_event_bus` + `event_bus.dart`)
- 带进度 + 可取消 → **模式 E** (`pattern_e_progress`)
- 日志 / tracing → 见手册 §15.5（本库未单列, 与模式 I 同型）

## Step 2B: Dart 发起调用 — 数据规模?

### 数据小（单值 / 几 KB 以内）

- 函数体 <100us 且无 IO → **模式 A** (`pattern_a_sync`)
- 有 IO / 阻塞 → 进 Step 3

### 数据中（KB ~ 单 MB）

- 普通 DTO struct → **模式 B** (`pattern_b_async`)
- 动态 schema → **模式 R** (`pattern_r_json`)
- 列表分页 → **模式 K** (`pattern_k_pagination`)

### 数据大（> 10 MB）

- 在内存里 → **模式 G** (`pattern_g_bytes`, 单次返回 / 分块)
- 在磁盘文件 → **模式 S** (`pattern_s_file_handoff`)

## Step 3: 异步 — 控制需求?

- 简单一次性 → **模式 B**
- 用户可取消 → **模式 D** (`pattern_d_cancel`)
- 取消 + 进度 → **模式 E**
- 需要在多次调用间保持 Rust 端状态 → **模式 F** (`pattern_f_opaque`)
- Rust 内部需要调 Dart (依赖反转) → **模式 M** (`pattern_m_callback`)

## Step 4: 横切关注?

- **重试 / 退避**: 包一层 `retry.dart` (Dart, 含 `totalTimeout` + `CancelToken` + `onRetry`), 配合 `pattern_l_idempotent.rs` (Rust LRU+TTL 缓存)
- **短时请求合并**: `coalescing.dart` (失败不缓存, 提供 `peek` / `clear`)
- **并发限流**: `pool.dart` (Dart, 支持 `tryRun` / `cancelWaiters` / `close`) + `pattern_n_semaphore.rs` (Rust 命名池注册表)
- **后台 isolate**: `isolate.dart`
- **启动 / 单例 / 关闭**: `pattern_p_singleton.rs` (含 `transport_shutdown` 供 hot-restart)
- **测试 / mock**: `pattern_q_mock.rs` + `mock.dart` (`withMocks` per-test 隔离)
- **panic 安全 / 防 FFI 崩溃**: **模式 T** (`pattern_t_panic_safety`, 任何 `tokio::spawn` 内部都该过 `catch_panic_async`)
- **结构化并发 / fan-out / race**: **模式 U** (`pattern_u_structured`, 子任务统一取消)

## Step 5: 跨边界硬约束?

- **任何 `tokio::spawn` 里的 panic 会爬过 FFI → UB**: 用模式 T 包一层。
- **N 个子任务 + 任一失败要级联取消**: 用模式 U; 不要 `tokio::spawn` 后忘记 join。
- **进程级单例 + 热重启**: `pattern_p_singleton` 暴露 `transport_shutdown`, Dart 侧 `_disposed` 标志 + `event_bus.dispose()` 配合。
- **需要 request_id / deadline / idempotency / traceparent / locale 中的任一**: **模式 V** (`pattern_v_request_meta` + `request_context.dart`) 作可选第一参数; 不污染业务签名。

## Step 6: 数据形状?

新加一个 pattern 时, 先按形状分流: **强类型 (绝大多数) / JSON (动态) / 字节或文件 (大块) / 元数据 (横切)**。详见 [`data-shapes.md`](./data-shapes.md)。

## 常见场景 → 模式映射

| 场景 | 模式组合 |
|---|---|
| 文本输入实时检查文本（如违禁词） | A 或 B (取决于耗时) |
| 加载用户资料 | B + J（避免列表重复请求） |
| 导入 200MB 数据库文件 | S |
| 摄像头帧解码后送 Rust 计算特征 | G (分块) 或 S（落盘后再传路径）|
| 后台同步任务有进度条 | E |
| 通知中心: Rust 跨多页推送红点 | I |
| 加密 / 解密大段文本 | B + O（放后台 isolate）|
| 用户长按"导出 PDF", 可中途取消 | E |
| 查询接口幂等 | L (retry.dart + idempotent.rs) |
| 单元测试 widget, 不想真启动 Rust | Q |
| 维持一个 SQLite 连接句柄跨多次查询 | F |
| Rust 需要从 Dart 拿密钥 / token | M |
| 接入第三方 JSON API | R (Rust 不解码) |
| 一段计算每秒 60 次（每帧调）| A，但要小心 §6.13.1 注意事项 |
| 任何 `tokio::spawn` 内的代码 (防 panic 穿 FFI) | T (`catch_panic_async`) |
| 并行 N 子任务 / 任一失败级联取消 | U (`transport_sample_fanout`) |
| 多候选竞速取第一个成功 | U (`transport_sample_race`) |
| 热重启时清理 Rust 端单例资源 | P (`transport_shutdown`) |
| 大文件落盘后给 Rust + 完整性校验 | S (`expected_sha256` 参数) |
| 端到端追踪 / 超时预算 / 幂等键 | V (`TransportRequestMeta` + `RequestContext`) |
