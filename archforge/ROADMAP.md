# ArchForge ROADMAP

本文档跟踪 Step 0–6 的完成度。原则：**先有用例，再有抽象（Rule of Three）**。

---

## ✅ Step 0 — 立宪 (Done)

- [x] `ARCHITECTURE_INVARIANTS.md` 5 条不变量
- [x] 反模式清单
- [x] PR 拒绝规则

## ✅ Step 1 — 最小核 `archforge-kernel` (Done)

- [x] `AppError`（narrow enum + `#[non_exhaustive]` + 收敛 io / serde_json 错误）
- [x] `Context`（trace_id / actor / locale / deadline / idempotency_key）
- [x] `Timestamp`（ms since epoch，零外部依赖）
- [x] `arch_newtype!` 宏（String / Uuid 两种，自动 `serde` 校验）
- [x] `DomainEvent` trait + `OutboxSink` trait
- [x] Capability marker traits（`ReadOnly`, `Transactional`, `BulkLoadable`）

**冻结条款**：kernel 1.0 释放后 12 个月内不接受 breaking change。

## ✅ Step 2 — 第一条垂直切片 `auth` (Done)

完整链路：`contract-auth → domain-auth → app-auth → infra-auth-{memory,jsonfile}`，外加 `examples/auth-cli` 装配演示。

- [x] `contract-auth`：DTO/Cmd/Query/Event/Port/Error
- [x] `domain-auth`：富 User 模型（Always-Valid，私有字段）
- [x] `app-auth`：`CreateUserUseCase` / `FindUserUseCase`
- [x] `infra-auth-memory`：DashMap 后端
- [x] `infra-auth-jsonfile`：tokio + serde_json 文件后端（跨平台、零原生依赖）
- [x] `conformance`：同一组 property test 跑遍两个后端
- [x] `examples/auth-cli`：Cargo feature 切换 backend，业务代码零修改

## 🚧 Step 3 — 抽公共积木 (Next)

等待第 2、第 3 条垂直切片出现后再抽，避免过早抽象。候选清单：

- [ ] `archforge-domain-rich`：通用富领域宏（`#[derive(Entity, AggregateRoot)]`）
- [ ] `archforge-domain-typestate`：类型态状态机宏
- [ ] `archforge-domain-lite`：贫血模型 + builder
- [ ] `archforge-app-cqrs`：`CommandBus` / `QueryBus` 通用 trait
- [ ] `archforge-app-workflow`：本地 Saga 编排
- [ ] `archforge-uow`：`UnitOfWork` 抽象（**只在第三个跨聚合根用例出现时引入**）
- [ ] `archforge-outbox`：通用 outbox + 异步消费
- [ ] `archforge-observe`：tracing/metrics layer（Tower 风格装饰器）

候选垂直切片（等出现真实需求时再做）：

- [ ] `examples/notes`：rich domain + CQRS + stream
- [ ] `examples/wallet`：typestate + workflow + telemetry

## 🚧 Step 4 — 护城河工具链 (Pending)

- [ ] `deny.toml`（已起草，待 `cargo-deny` CI 接入）
- [ ] `cargo-public-api` 卡 contract crate 的 API 变更
- [ ] `cargo-semver-checks` 卡 semver 兼容性
- [ ] `cargo-mutants` kernel 层变异测试（目标 ≥ 95%）
- [ ] `cargo-fuzz` 覆盖 DTO 反序列化、newtype 构造
- [ ] `criterion` 性能基线（回归 > 5% 报警）

## 🚧 Step 5 — 脚手架 (Pending)

- [ ] `cargo-generate` 模板：minimal-clean / ddd-standard / cqrs-stream / enterprise-full
- [ ] `archforge-cli` 命令：`new-context`, `new-adapter`, `check-invariants`

## 🚧 Step 6 — 治理 (Pending)

- [ ] 每月兼容性矩阵 CI（4 domain × 3 app × 5 infra × 2 ffi）
- [ ] 每 crate 独立 `OWNERS` + `CHANGELOG.md`
- [ ] RFC 流程
- [ ] ADR 模板

## 🚧 Flutter / FFI (Pending)

- [ ] `archforge-ffi-bridge`：`flutter_rust_bridge` v2 + `catch_unwind` 隔离墙
- [ ] `archforge-ffi-stream`：Rust Channel ↔ Dart `Stream` 双向绑定
- [ ] `flutter/packages/archforge_kit`：Dart 包 + 生成的 binding + Riverpod 模板

---

## 决策日志

- **2026-05-14 MVP**：选择 `auth` 作为第一条垂直切片（业务边界清晰、不含跨聚合根事务、便于 conformance）。选择 `infra-auth-jsonfile` 而非 sqlite 作为第二 Adapter，理由：零原生依赖、跨平台稳定、足以证明双 Adapter LSP 一致性；真实 sqlite Adapter 留到 Step 3 与 CQRS 一起来。
- **UoW 延后**：MVP 范围内无跨聚合根事务，引入 UoW 是过早抽象。等第三个用例需要时再抽。
