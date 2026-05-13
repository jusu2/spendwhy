# Transport 通用陷阱清单

与工程手册 §6.13.18 互为补充。

## 跨边界共性

1. **不要把 Rust `Result` 错误信息透传到 UI**。统一过 `TransportError`。
2. **不要返回深嵌套结构** (`List<List<Map<String, Dto>>>`)。改成列式 (模式 K) 或分页。
3. **不要在持锁状态下 await Dart 回调** (模式 M)。死锁高发。
4. **不要在 hot loop 里调 sync FFI 几百次**。预计算成 batch (模式 A → 模式 K)。
5. **DateTime / 时区**: 统一 i64 毫秒时间戳跨边界, UI 层再格式化。

## 模式 A (sync) 陷阱

- 函数体不可有任何阻塞（DB / fs / Mutex 重锁）→ 阻塞 UI 线程。
- 不能返回大数据 (>64KB) → 改模式 B 或 G。

## 模式 B (async future) 陷阱

- 一旦决定异步, 必须支持取消 → 改模式 D。
- 默认超时: Dart 侧用 `Future.timeout(...)` 或 Rust 侧 `transportSampleWithTimeout`。

## 模式 C / E / I (stream) 陷阱

- Dart 端 cancel subscription, Rust 端要能感知 (`sink.add(...).is_err()`)。
- 不要在 hot loop 里 `sink.add` 而不让出 → `tokio::task::yield_now().await`。
- `broadcast::error::Lagged` 不是错误, 是慢消费者: 选 drop 还是 panic 要在业务侧决策。

## 模式 D / E (cancel) 陷阱

- `CancelHandle` 是 RustOpaque, **必须 Dart 端持有引用直到任务结束**, 否则 GC 后 cancel 无效。
- 不要把同一 `CancelHandle` 用于多个任务: 取消一个会取消全部。

## 模式 F (opaque) 陷阱

- 内部状态自己加 `Mutex`/`RwLock`, FRB 不替你同步。
- 不要塞进 Provider 全局 → 页面退出忘 dispose, 进程级泄漏。
- `dispose()` 后所有方法应返回 `Conflict`, 不能 panic。

## 模式 G (bytes) 陷阱

- FRB 2.x `Vec<u8>` 已自动 `Uint8List`; 不需要 `ZeroCopyBuffer`。
- 单次返回 >50MB → 改模式 S。
- 分块流要约定 chunk 大小, 不要让 Rust 自适应 (Dart 端 backpressure 不好处理)。

## 模式 H (duplex) 陷阱

- FRB 不直接支持 Dart→Rust stream 入参; 用 Opaque 句柄 + 方法调用。
- 注意 sink 注册前 submit 命令的丢失: 在 Rust 侧 buffer 一段时间或拒绝。

## 模式 I (event bus) 陷阱

- 总线必须在进程启动时一次性初始化 (模式 P); 不要懒初始化在某个具体调用里。
- broadcast capacity 设置: 太大占内存, 太小慢订阅者频繁 lag。

## 模式 J (coalescing) 陷阱

- 只合并"完全相同的请求", 用业务 key 决定 (不仅是参数哈希)。
- 失败结果也会被等待者拿到 → 是否要在失败时立即剔除? 默认实现是会剔除 (whenComplete)。

## 模式 K (pagination) 陷阱

- cursor 不要用 offset (列表中间插入会跳行); 用 keyset (created_at, id) 的不透明 base64。
- 一页大小 32~128, 不要 1000。

## 模式 L (retry / idempotent) 陷阱

- `retry` 不要重试 `canceled` / `invalid_argument` / `not_found` / `conflict`。
- Rust 侧 idempotency cache 必须有 TTL / 上限, 否则内存泄漏。

## 模式 M (callback) 陷阱

- 不要在 Rust 持锁状态 `await dart_callback(...)`。
- Dart 回调里不要再调 Rust 同一入口 → 重入死循环。

## 模式 N (semaphore) 陷阱

- 进程级 Semaphore 跨业务共享会引发资源饥饿; 按业务分多个 Semaphore。

## 模式 O (isolate) 陷阱

- 任务总耗时 <50ms 时, isolate spawn + RustLib.init 开销得不偿失。
- 子 isolate 的 `StreamSink` 跨 isolate 传递不稳, 流式工作留在主 isolate。
- 子 isolate 的 panic 不会自动上报到主 isolate; 要用 `try/catch` 捕获并显式 rethrow。

## 模式 P (singleton) 陷阱

- `#[frb(init)]` 钩子里不要做昂贵 IO (DB 迁移、远程拉取)。
- 单例的 dispose / shutdown 协议要明确, 否则 hot restart 资源不释放。

## 模式 Q (mock) 陷阱

- 不要在 widget test 里走完整 Rust 桥; 用接口分层 + `TransportMockRegistry`。
- mock 在 integration_test 里没意义 (那里就是要测真桥)。

## 模式 R (JSON) 陷阱

- Rust 不解码 = Rust 不能假设结构。所有"我要从 JSON 里读 X"的逻辑放 Dart。
- 不要把模式 R 用作"懒得写 DTO 的捷径"; schema 稳定就写 struct。

## 模式 S (file handoff) 陷阱

- 双方约定 ownership: 谁负责删除。本库默认 Rust 删 (受 `delete_after` 参数控制)。
- 权限: iOS sandbox 的 NSTemporaryDirectory 跨进程访问规则不同, 测试机和真机表现可能不一致。
- 校验: 大文件传输前后跑 sha256, 不要假设文件完整。本库 `expected_sha256` 参数会在不匹配时返回 `conflict`。

## 模式 T (panic safety) 陷阱

- `flutter_rust_bridge` 只对 `pub` 入口加 `catch_unwind`; `tokio::spawn` 出去的代码不在保护范围内 → 自己用 `catch_panic_async` 包一层。
- `AssertUnwindSafe` 是一个 promise: 你保证即使 panic 也不会留下半成品状态 (持锁中的 `std::sync::Mutex` 会 poison)。能用 `parking_lot::Mutex` 就用。
- panic 信息只回传 `internal` code, **不要**把 message 透传 Dart, 避免泄漏路径 / 凭证。日志里保留原文。

## 模式 U (structured concurrency) 陷阱

- 不要 `tokio::spawn` 后忘记 join: 那是**无结构并发**, 任务会脱离 future tree。用 `join_all` / `select_all`。
- drop `JoinHandle` 不会取消任务; 必须显式 `CancelToken::cancel`。
- fan-out 上限要硬约束 (本库默认 32), 防止意外提交大批任务。
- 子任务 `Result` 失败时, **立即 `cancel()`** 让其余 inflight 任务有机会观察到。
- "尽快返回第一个成功"用 `select!` / `select_all`; "全部完成"用 `join_all` + 在收集阶段判第一个 err。

## Dart `whenComplete` 陷阱 (coalescing 实战教训)

- `future.whenComplete(() => map.remove(k))` 当箭头返回 `Future<V>?` 时, `whenComplete` 会 **等待该 future** 才标记完成 → 同一 future 等自己 → 死锁。
- 修复: 用 block body `() { map.remove(k); }` 显式返回 `void`。本库 `coalescing.dart` 改用 `Completer + _drive` async helper 同时解决。
