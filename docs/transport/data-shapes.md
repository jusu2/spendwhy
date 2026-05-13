# 数据形状 (Data Shapes) 决策指南

> 一句话: **Flutter → Rust 的数据形状不是越通用越好**。本库刻意按形状分流, 强类型走强类型, 动态走 JSON, 大块走字节/路径; 横切语义 (谁、多久、能否重试) 再叠一层元数据。

---

## TL;DR — 三类形状 + 一层横切

| 形状 | 何时 | 用什么 |
|---|---|---|
| **形状已知且稳定** (绝大多数) | 字段名/类型在编译期就定 | 各 `pattern_*` 自定义强类型 DTO |
| **形状未知 / 动态** | 第三方 API / 用户自填 schema | **模式 R** `pattern_r_json` JSON 字符串透传 |
| **数据大 / 二进制** | >几 MB 或非结构化字节 | **模式 G** `Vec<u8>` (≤50MB) / **模式 S** 文件路径 + sha256 |
| **横切元数据** (与形状正交) | 任何调用都可能需要追踪、超时、幂等 | **模式 V** `TransportRequestMeta` 作可选第一参数 |

---

## 为什么不做"万能 DTO"

设计一个 `TransportEnvelope { kind: String, payload: Vec<u8> }` 看似优雅, 实际有四宗罪:

1. **丢失类型安全**: 编译期能查的契约变成运行期分发, FRB 自动生成的类型镜像失效。
2. **双重序列化**: Dart→bincode→Rust→bincode→业务结构, 字节多走两次。
3. **错误信息劣化**: schema 不匹配从 "字段 X 类型错" 变成 "payload 第 17 字节解析失败"。
4. **观测变难**: tracing 看不到字段名, 排查只能 dump 原始字节。

**强类型 DTO + JSON 逃生舱 + 文件大数据通道** 是已经被 gRPC / Tonic / Protobuf well-known-types 验证过的分流方式。本库照搬。

---

## 决策树: 我现在要送的数据属于哪类?

### Step 1: 数据大小?

```
≤ 32 KB  → Step 2 (结构化路径)
32KB ~ 50MB 二进制 → 模式 G (Vec<u8> / 分块 stream)
> 50MB 或在磁盘 → 模式 S (文件路径 + sha256 校验)
```

### Step 2: 形状稳定吗?

```
是 (编译期 schema)  → Step 3
否 (运行期 schema, 嵌套深 / 字段名动态) → 模式 R (JSON 字符串)
```

### Step 3: 哪个 pattern 匹配语义?

参见 `decision-tree.md` (按调用方向 / 是否可取消 / 是否分页 等维度)。

### Step 4 (横切): 这个调用需要追踪 / 超时 / 幂等吗?

```
否, 内部一次性 fire-and-forget → 不加元数据
是, 任何一项 → 在签名第一个位置加 meta: TransportRequestMeta (模式 V)
```

---

## 模式 V: `TransportRequestMeta` 字段语义

| 字段 | 类型 | 用途 | Dart 端 |
|---|---|---|---|
| `request_id` | `String` (1..=128 ASCII) | 端到端追踪 ID | `RequestContext.create()` 自动生成 UUID v4 |
| `idempotency_key` | `Option<String>` | 接收方去重 (配合模式 L) | `.withIdempotency('apply-$noteId')` |
| `budget_ms` | `Option<u64>` | 剩余预算 (ms); 到期立刻 `timeout` | `Deadline(timeout).remainingMs` |
| `trace_parent` | `Option<String>` (W3C 55 char) | 分布式追踪头 | `newTraceParent()` |
| `locale` | `Option<String>` (BCP-47) | 本地化 | `.withLocale('zh-CN')` |
| `attempt` | `u32` (≥1) | 1=首次 / 2+=重试 | `.bumpAttempt()` |
| `source` | `Option<String>` (≤64) | 调用来源标识 | `RequestContext.create(source: 'ui.x')` |

### 关键设计取舍

- **`budget_ms` 而非 `deadline_at_epoch_ms`**: wall-clock 在两端会跳 (NTP 同步、用户改时), 单调时钟才稳。Dart 端用 `Stopwatch` 算剩余, Rust 端落地为 `Instant::now() + Duration`。
- **`trace_parent` 不解析仅转发**: 与 W3C Trace Context 规范一致, 上下游中间件负责解析。
- **`attempt` 由调用方传**: Rust 不能凭一个 `request_id` 推断重试 (中间可能换了 process); Dart `retry()` 调 `bumpAttempt()`。
- **校验严格**: 字段长度上限、ASCII 限制、traceparent 格式 — 防 DoS, 防垃圾日志, 防被注入到 tracing tag 里搞乱观测。

---

## 把模式 V 接入现有模式的两种姿势

### A) 调用方包一层 (推荐, 不动现有签名)

```dart
// 业务层 helper
Future<T> withMeta<T>(
  RequestContext ctx,
  Future<T> Function(TransportRequestMeta meta) call,
) async {
  await transportSampleValidateMeta(meta: ctx.freeze()); // 早抛
  return call(ctx.freeze());
}

// 调用
final result = await withMeta(ctx, (meta) =>
    transportSampleApplyOnce(
      idempotencyKey: meta.idempotencyKey ?? '',
      payload: payload,
    ));
```

### B) 在新 pattern 签名里把 meta 作首参 (业务级模式)

```rust
pub async fn my_service_apply(
    meta: TransportRequestMeta,
    payload: MyPayload,
) -> Result<MyResult, TransportError> {
    meta.validate()?;
    let started = std::time::Instant::now();
    // ... do work ...
    meta.check_deadline(started)?;
    Ok(result)
}
```

两种都可以。**本库标本 (pattern_a..u) 不强制接 meta**, 保持每个标本最小; 真实业务的新模式按需选 (A) 或 (B)。

---

## 反模式 (Don't)

1. **不要把业务字段塞进 `source`**: `source` 是粗粒度调用源 (`ui.list`, `bg.sync`), 不是参数槽。
2. **不要在 Rust 内部 mutate `meta`**: 它是不可变快照, mutate 会让 tracing 字段对不上。
3. **不要复用 `request_id` 做"会话 ID"**: 一次调用 = 一个 id。多次调用形成会话用业务级 session id (在 payload 里)。
4. **不要把 `idempotency_key` 当 cache key**: 它是"同一逻辑请求重发的标记", 不是"缓存键"。缓存键由业务定。
5. **不要让 `budget_ms` 替代 `tokio::time::timeout`**: meta 给的是**总预算**, Rust 内部具体子任务还应有自己的小超时, 避免单步卡死。
