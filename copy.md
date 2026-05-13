# Flutter + Rust 极致性能架构蓝图（通用模板）

版本: v1.0
日期: 2026-05-02
适用范围: iOS / Android 的 Flutter + Rust 应用（当前项目与后续所有同类项目）

---

## 1. 文档目标

本文档是统一的研发宪章，目标不是“写得能跑”，而是“长期可演进、海量数据可承载、性能可度量、稳定可发布”。

本文档覆盖:

1. 架构哲学与边界划分
2. FFI 调用标准（批量化、异步化、少拷贝）
3. 数据模型与存储策略
4. 状态管理与 UI 性能规则
5. 并发、取消、背压机制
6. 指标驱动的性能工程方法
7. 工程化、CI/CD、安全发布规范
8. 通用模板目录与执行清单

---

## 2. 最高设计哲学（不可违反）

### 2.1 五条铁律

1. Rust 是后端内核，Flutter 是交互与渲染层。业务核心、算法、数据访问统一在 Rust。
2. FFI 边界必须粗粒度。一次调用处理一批数据，禁止细粒度高频往返。
3. UI 线程不可做重活。超过 1ms 的任务默认异步，不允许同步阻塞帧渲染。
4. 跨边界数据必须扁平化与类型化。优先列式、连续内存、零拷贝通道。
5. 指标先于优化。所有性能优化必须基于 trace、benchmark、回归门禁，不凭感觉。

### 2.2 价值排序

1. 正确性 > 可观测性 > 性能 > 便利性
2. 架构稳定性 > 局部技巧
3. 可回归验证的改进 > 一次性“玄学加速”

---

## 3. 分层架构（5 层 + 1 桥）

```
Presentation (Flutter Widgets / Riverpod)
	 ↓
Application (UseCase / Controller / Orchestration)
	 ↓
FFI Bridge (flutter_rust_bridge, coarse-grained API)
	 ↓
Domain Core (Rust pure logic, policy, algorithm)
	 ↓
Infrastructure (Rust DB/Network/Cache/Index)
	 ↓
Platform Adapters (Android/iOS only when necessary)
```

### 3.1 各层职责

1. Presentation
	- 只做渲染、交互响应、局部状态选择订阅。
	- 不写业务规则、不直连数据库、不直连 HTTP。
2. Application
	- 负责编排调用、事务边界、防抖节流、错误映射。
	- 不存放复杂算法。
3. FFI Bridge
	- 仅承载类型边界与调用协议。
	- API 必须是“高价值、低频次、可批处理”的用例接口。
4. Domain Core (Rust)
	- 纯逻辑、策略、核心算法。
	- 可独立单测、可基准测试。
5. Infrastructure (Rust)
	- 数据库、网络、缓存、检索、日志、配置。
	- 通过 trait 对 Domain 暴露能力。
6. Platform
	- 仅在 Rust 无法直接触达能力时使用 Kotlin/Swift。

---

## 4. FFI 标准（批量化 / 异步化 / 少拷贝）

### 4.1 调用分级制度

1. Sync 档
	- 仅用于超轻量纯计算（目标 < 100us）。
	- 使用 `#[frb(sync)]`。
	- 禁止 IO、禁止锁等待、禁止大内存分配。
2. Async 档（默认档）
	- DB 查询、网络调用、复杂计算统一 async。
	- Dart 侧通过 `await` 获取结果，UI 不阻塞。
3. Stream 档
	- 长任务、增量事件、实时订阅。
	- 用于进度上报、变更流、搜索流。
4. Zero-Copy 档
	- 大块二进制、向量、图像、音频、张量数据。
	- 使用 `ZeroCopyBuffer` 减少内存复制。

### 4.2 禁止事项

1. 禁止在 Dart 循环中逐条调用 FFI。
2. 禁止跨 FFI 传递 `dynamic/Map` 作为主数据通道。
3. 禁止把 JSON 字符串作为主要互操作协议。
4. 禁止在同步 API 中做任何不可控耗时行为。

### 4.3 API 设计模板

```rust
#[frb(sync)]
pub fn quick_score(x: i64, y: i64) -> i64 { ... }

pub async fn list_entities(filter: Filter, page: Page) -> Result<EntityBatch> { ... }

pub fn watch_entities(sink: StreamSink<EntityEvent>) -> Result<()> { ... }

pub fn transform_blob(input: ZeroCopyBuffer<Vec<u8>>) -> ZeroCopyBuffer<Vec<f32>> { ... }
```

---

## 5. 数据建模标准（扁平化 + 列式 + 主键引用）

### 5.1 DTO 原则

1. 跨边界对象必须扁平结构，减少嵌套层级。
2. 子对象关联优先“主键引用 + 二次批量查询”。
3. 高频聚合字段预计算（如计数、状态位）。
4. 时间统一毫秒时间戳（i64）。

### 5.2 列式批量结构

```rust
pub struct EntityBatch {
	 pub ids: Vec<i64>,
	 pub created_ms: Vec<i64>,
	 pub titles: Vec<String>,
	 pub scores: ZeroCopyBuffer<Vec<f64>>,
}
```

适用场景:

1. 大列表首屏加载
2. 分页连续滚动
3. 可视化图表
4. 批量统计展示

---

## 6. 状态管理与 UI 规则

### 6.1 架构原则

1. 单向数据流: Command -> Rust -> Event Stream -> State -> UI。
2. 写操作不触发全量 reload，依赖增量事件合并。
3. 组件只订阅所需字段（select/watch granularity）。
4. 不允许“全表加载 + 内存过滤”作为常态。

### 6.2 推荐组合

1. Riverpod 2（或等价可精细订阅方案）
2. 不可变集合（减少拷贝与误修改）
3. 分页游标 + 懒加载

### 6.3 UI 性能红线

1. 任何单帧任务预算不超过 16ms（60fps）。
2. 大列表使用构建器与固定高度优化。
3. 图片/数据解码与重计算不进入主线程。

---

## 7. 存储与查询标准（Rust 统一持有）

### 7.1 存储选型

1. 关系型: sqlite + SQLCipher（加密）
2. KV/嵌入: sled 或 redb
3. 全文检索: FTS5
4. 向量检索: sqlite-vec / 等价库

### 7.2 查询与分页

1. 必须使用 Keyset Pagination（游标分页）。
2. 禁止在海量数据上使用 OFFSET 作为核心分页方案。
3. 常用查询路径必须建立复合索引。
4. 批量写入统一走事务。

### 7.3 仓储接口规范

```rust
#[async_trait]
pub trait EntityRepo: Send + Sync {
	 async fn list(&self, filter: Filter, page: Page) -> Result<Vec<EntityDto>>;
	 async fn add(&self, input: NewEntity) -> Result<i64>;
	 fn watch(&self) -> broadcast::Receiver<EntityEvent>;
}
```

---

## 8. 并发模型（吞吐、取消、背压）

### 8.1 Rust 侧

1. IO 密集: tokio async。
2. CPU 密集: rayon 或 `spawn_blocking`。
3. 共享读多写少场景优先无锁或低锁设计。

### 8.2 取消机制

每个长任务都必须可取消:

```rust
tokio::select! {
	 _ = cancel_token.cancelled() => return Err(Error::Cancelled),
	 result = do_work() => result,
}
```

### 8.3 背压机制

1. Stream channel 必须有容量上限。
2. 明确定义 overflow 策略（丢弃旧值/合并/阻塞）。
3. 上游速度不得无限制压垮下游 UI。

---

## 9. 指标驱动体系（Observability First）

### 9.1 三层观测

1. Flutter 层
	- DevTools timeline、帧时间、内存、GC。
2. FFI 边界
	- 每次调用记录开始/结束/耗时/入参规模。
3. Rust 内核
	- tracing span、关键路径耗时、错误分布。

### 9.2 基准测试制度

1. Rust: criterion benchmark，核心算法必须有基线。
2. Flutter: profile 模式下首屏、滚动、交互压测。
3. E2E: integration_test + 关键场景回归。

### 9.3 发布门禁（必须全绿）

1. P95/P99 延迟不劣化。
2. 启动时间与内存不劣化。
3. 核心场景帧率无明显回退。

---

## 10. 工程化规范（可复制到所有项目）

### 10.1 代码组织

```
app_root/
  lib/
	 app/
	 features/
	 shared/
	 src/rust/          # frb generated
  rust/
	 src/
		api/
		domain/
		infra/
	 benches/
  test/
  integration_test/
  flutter_rust_bridge.yaml
  analysis_options.yaml
```

### 10.2 质量工具

1. Flutter: analyze + unit test + integration test。
2. Rust: fmt + clippy(-D warnings) + test + bench。
3. 提交钩子: 本地 pre-commit 自动执行最小质量门禁。

### 10.3 CI/CD 规范

1. 平台矩阵构建（Android/iOS）。
2. 自动化测试全链路。
3. 版本、变更日志、产物归档自动化。

---

## 11. 安全与发布基线

1. Release 包必须使用正式签名，禁止 debug signing。
2. 敏感数据必须加密存储，密钥分层托管。
3. 依赖安全扫描纳入 CI（Rust 与 Dart 双栈）。
4. 最小权限原则，清理不必要系统权限。
5. 线上崩溃与性能事件必须可追踪与回放定位。

---

## 12. 性能预算与 SLO（建议基线）

1. 冷启动: < 1.5s（Profile 基准机型）
2. FFI 同步调用: P99 < 0.1ms（轻量函数）
3. FFI 异步调用: P99 < 16ms（交互关键路径）
4. 大列表滚动: 稳定 60fps（高端机向 120fps 逼近）
5. 1 万条数据常驻内存: 受控且无异常抖动

说明: 具体阈值按业务与机型分层设定，本节作为默认起点。

---

## 13. 当前项目改造路线（执行版）

### 阶段 A（立即）

1. 用 Stream 驱动列表状态，消除“写后全量 reload”。
2. 列表查询改成游标分页。
3. 建立 FFI 调用耗时埋点。

### 阶段 B（短期）

1. 持久层统一迁移到 Rust（含事务与索引策略）。
2. 引入批量 DTO / 列式通道。
3. 对核心算法建立 criterion 基线。

### 阶段 C（中期）

1. 接入完善的线上观测（崩溃 + 性能）。
2. 完成 CI 质量门禁与发布流水线。
3. 完成安全基线闭环（签名、加密、依赖扫描）。

---

## 14. 团队执行守则

1. 任何新功能先写 Rust API 设计，再写 Flutter 页面。
2. 任何性能问题先出数据，再提方案。
3. 任何跨 FFI 设计评审必须回答三问:
	- 能否批量化？
	- 能否异步化？
	- 能否少拷贝？
4. 任何上线前必须通过 SLO 与回归门禁。

---

## 15. 结语

这套方案的本质是:

1. 用 Rust 保证核心能力的确定性、吞吐和一致性。
2. 用 Flutter 提供高质量、低耦合、可快速迭代的交互层。
3. 用指标系统把“性能”从口号变成工程事实。

当项目规模上升、数据量上升、团队人数上升时，这份架构仍然成立，并且越到后期价值越大。

本文件即本项目及后续 Flutter + Rust 项目的标准开发哲学与框架指导。

