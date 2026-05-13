# ArchForge — Rust + Flutter 工业级架构积木库

- **文档版本**：1.3（整理合并版）
- **项目版本**：0.1.0（MVP Design）
- **状态**：已评审，可进入实现阶段
- **作者**：hongsickS
- **日期**：2026-05-14
- **审批流程**：架构评审委员会 / 社区 RFC
- **相关文档**：`docs/ADR/*.md`、`docs/C4_Model/`、`docs/ROADMAP.md`

---

## 1. 执行摘要 (Executive Summary)

**ArchForge** 是一个**生产就绪、模块化、可组合、可演进**的 Rust + Flutter 架构积木库（Architecture Building Blocks Toolkit）。

它解决中大型跨平台项目中的常见痛点：**技术栈强耦合、业务逻辑分散、可维护性随时间衰退、团队认知不一致、新项目启动成本高**。

**核心定位**：不是“大而全框架”，而是**“架构乐高工厂”**——提供高质量、可插拔的积木组件，让开发者根据业务复杂度、性能要求、团队规模、部署环境等上下文，灵活组装出最优架构方案。

**关键特性**：

- **Rust 核心**：强类型、零成本抽象、内存安全、并发安全。
- **Flutter 无缝集成**：通过 `flutter_rust_bridge` 实现高性能跨平台。
- **多范式支持**：Clean / Hexagonal + 可选 DDD / CQRS / Event Sourcing / Data-Oriented Design。
- **真正即插即用**：Cargo features + Ports & Adapters + 丰富模板 + 脚手架。
- **工业级工程实践**：100% 可测试、显式依赖、全面可观测、Semantic Versioning、ADR 驱动决策。

**预期价值**：

- 新项目架构搭建时间缩短 50–70%。
- 长期维护成本降低 40%+。
- 技术演进风险可控（DB 迁移、UI 框架更换等）。
- 团队 onboarding 加速，代码一致性提升。

---

## 2. 目标与非目标 (Goals & Non-Goals)

### 2.1 业务目标

- 赋能开发者快速构建高质量、可长期维护的 Rust + Flutter 应用。
- 成为 Rust 社区 Flutter 混合开发领域的参考架构标准。

### 2.2 技术目标

- 支持从 **Minimal Clean** 到 **Enterprise Full DDD + CQRS + ES** 的平滑谱系。
- 强制良好实践：依赖倒置、Always-Valid Domain Model、显式架构。
- 优秀 DX：文档、示例、模板、CLI。
- 高质量交付：测试覆盖、性能基准、安全扫描、Observability。

### 2.3 非目标

- 提供完整业务领域模型或 UI 组件库。
- 强制单一架构风格（支持 Minimal 路径）。
- 替代底层优秀库（SQLx、SeaORM、Riverpod、Tokio 等）。
- 成为封闭生态（欢迎社区贡献适配器）。

---

## 3. 核心设计原则 (Design Principles)

1. **依赖倒置 (DIP)**：Domain / Application 只依赖抽象（Ports），外层提供适配器。**理由**：解耦业务与技术细节，便于测试和演进。
2. **领域优先与统一语言**：使用 Rust 强类型（Newtype、Enum、Pattern Matching）精准表达业务概念。**理由**：减少沟通损耗，提升模型准确性。
3. **Ports & Adapters（Hexagonal）**：核心业务居中，所有外部交互通过 Port 访问。**理由**：最大化灵活性与可测试性。
4. **Clean Architecture 分层**：洋葱模型，依赖始终向内。**理由**：关注点分离，变化速率不同的代码隔离。
5. **零成本抽象与性能导向**：优先具体类型 + Generics，慎用 `Box<dyn Trait>`。**理由**：保留 Rust 核心优势。
6. **可组合性与渐进式复杂度**：Cargo features + 模块化 crate。**理由**：避免一刀切，适配不同规模。
7. **可测试性与可观测性优先**：所有核心组件易 Mock、易 Property-Based 测试。**理由**：生产可靠性的基础。
8. **显式优于隐式**：少用 proc-macro 黑魔法，所有重大决策文档化于 ADR。**理由**：长期可维护。

### 3.1 乐高接口规范 (The Lego Studs)

模块之间能够任意组合，依赖三条**标准通信协议**：

1. **统一错误枚举 (`archforge-core-error`)**：所有积木的底层错误都会无损收敛到标准的 `AppError`。
2. **零依赖的 Port Traits (`archforge-core-ports`)**：通过纯 Trait 定义接口，实现依赖倒置。
3. **上下文透传 (`archforge-core-context`)**：极简的 `Context` 结构，携带 TraceID、用户身份等贯穿全层。

---

## 4. 高层架构视图 (C4 Model 摘要)

### Level 1 — 系统上下文

ArchForge Kit 作为架构中台，为多个 Bounded Context / 应用提供共享积木。

### Level 2 — 容器视图

- Flutter App（Presentation）。
- Rust Core（Domain + Application + Infra Adapters）。
- External Systems（DB、Cache、Auth Provider、第三方 API）。

### Level 3 — 组件视图

详见 `docs/C4_Model/component_diagrams/`（Mermaid / PlantUML）。

**默认分层依赖方向**：

```text
Presentation → Application → Domain (Ports) ↑ Infrastructure (Adapters)
```

**Bounded Context**：复杂领域建议独立 crate（如 `order-context`、`payment-context`），通过共享 `domain-core` 通信。

---

## 5. 仓库结构 (Repository Layout)

使用 Cargo Workspace 物理隔离层级，强制单向依赖。架构基建（Macros / Observability）独立为可发布的 crate。

```text
archforge/
├── Cargo.toml                          # Workspace + [workspace.dependencies]
├── justfile / Makefile                 # 标准化任务
├── README.md
├── LICENSE (MIT / Apache-2.0)
├── CONTRIBUTING.md
├── docs/
│   ├── ARCHITECTURE_DESIGN.md
│   ├── ADR/0001-xxx.md                 # Architecture Decision Records
│   ├── C4_Model/                       # Context / Container / Component
│   ├── diagrams/                       # Mermaid / PlantUML
│   ├── GLOSSARY.md
│   └── ROADMAP.md
├── templates/                          # cargo-generate 模板
├── examples/                           # 多场景示例
│   ├── minimal-clean/                  # 最小 Clean（适合简单项目）
│   ├── ddd-standard/                   # 标准 DDD
│   ├── full-cqrs-es/                   # CQRS + Event Sourcing
│   └── performance-hybrid/             # DoD + DDD 混合
├── core_blocks/                        # 🧱 与具体业务无关的核心积木
│   ├── arch-macros/                    # #[derive(Entity, AggregateRoot, DomainEvent)]
│   ├── arch-telemetry/                 # tracing / metrics
│   ├── arch-cqrs/                      # 内存级 Command / Query Bus 基础 trait
│   ├── arch-testing/                   # 母对象、proptest 工具
│   ├── domain-core/                    # 类型、Ports、事件、错误
│   ├── application-core/               # UseCase 基建、事务、验证
│   ├── event-sourcing/                 # Aggregate + Event Store + Outbox
│   └── infra-common/                   # Config、Mapper、Retry
├── business_blocks/                    # ⚙️ 业务层积木（以 Auth 为例）
│   ├── auth-domain/                    # 纯净 Rust：User 聚合根、Port Traits
│   └── auth-application/               # Use Cases，依赖 domain，无具体 I/O
├── infra_adapters/                     # 🔌 可插拔基础设施适配器
│   ├── infra-sqlx-postgres/
│   ├── infra-seaorm-sqlite/
│   ├── infra-redis-cache/
│   ├── infra-kafka/                    # Outbox 等
│   └── infra-mock-memory/              # 单测 & UI 联调
├── bridge_api/                         # 🌉 FFI 边界（暴露给 Flutter 的高阶接口）
│   ├── src/api.rs                      # 使用 #[frb] 接收 Command，调用 Application
│   └── src/dto.rs                      # 领域模型 → 视图模型
└── flutter/
    ├── packages/archforge_kit/         # Dart 包 + frb bindings + 示例
    └── app/
        └── lib/
            ├── features/auth/          # 对应后端 auth-application
            ├── core/bridge/            # frb 自动生成的 Dart binding
            └── shared/providers/       # Riverpod 状态容器
```

---

## 6. 积木全景：四维分层

每个维度提供一组可独立选用的 crate / feature，按**领域复杂度、调度模式、持久化目标、跨界与观测需求**分别取舍。

### 6.1 领域建模层 (Domain Modeling)

> 业务到底有多复杂？是简单 CRUD，还是复杂行业规则流转？

- 📦 **`archforge-domain-lite`（贫血模型 / CRUD 积木）**
  - **场景**：简单数据展示、配置读取、MVP 快速试错。
  - **特性**：普通 Rust Struct 作为数据容器，允许 Application 层直接修改其状态。
- 📦 **`archforge-domain-rich`（标准 DDD 积木）**
  - **场景**：中等复杂度核心业务（购物车、基础订单流程等）。
  - **特性**：`Entity` / `ValueObject` 抽象宏；字段私有化；仅通过业务方法变更状态（Always-Valid）。
- 📦 **`archforge-domain-typestate`（类型态状态机积木）**
  - **场景**：高安全要求、极复杂流转（金融交易、审批流）。
  - **特性**：基于 Rust 泛型状态机（Type-State），如 `Order<Unpaid>` 在**编译期**无法调用 `ship()`。

### 6.2 应用调度层 (Application Orchestration)

> UI 怎么调用业务逻辑？一把梭，还是命令/查询分离？

- 📦 **`archforge-app-service`（传统 Service 积木）**
  - **场景**：业务直接了当，主要为了复用逻辑。
  - **特性**：`Manager` / `Service` 结构体直接包含业务函数。
- 📦 **`archforge-app-cqrs`（CQRS 积木）**
  - **场景**：读多写少、UI 面板复杂、需要局部刷新（与 Flutter 状态管理天然契合）。
  - **特性**：轻量级 `CommandBus` / `QueryBus`；Command 写入并触发领域事件，Query 绕过 Domain 直接返回 DTO。
- 📦 **`archforge-app-workflow`（Saga / 工作流积木）**
  - **场景**：跨聚合根的长事务（下单同时扣库存、发券）。
  - **特性**：本地最终一致性，失败时自动调用补偿步骤（Rollback）。

### 6.3 基础设施与持久化层 (Infrastructure)

> 数据存哪？性能要多高？以后会不会换数据库？

- 📦 **`archforge-infra-memory`（内存 Mock 积木）**：UI 团队先行开发、单元测试，基于 `DashMap`。
- 📦 **`archforge-infra-sqlite-fast`（端侧本地库积木）**：离线优先 Flutter 客户端，基于 `rusqlite`，主打冷启动与零配置。
- 📦 **`archforge-infra-seaorm`（重型关系型 DB 积木）**：服务端或复杂本地关系数据分析，提供代码生成与完整 ORM。
- 📦 **`archforge-infra-kv`（键值缓存积木）**：用户偏好、会话状态，基于 `Sled` / `RocksDB`。
- 📦 **`archforge-infra-sqlx-postgres` / `archforge-infra-redis` / `archforge-infra-kafka`**：服务端常规适配器（关系型、缓存、消息总线 / Outbox）。

### 6.4 跨界通信与观测层 (Boundary & Observability)

> Rust 怎么优雅且安全地和 Flutter 通信？线上 Bug 怎么排查？

- 📦 **`archforge-ffi-bridge`（标准跨界积木）**：基于 `flutter_rust_bridge`，内置 `catch_unwind` 隔离墙，将 Panic 转为 Dart 异常。
- 📦 **`archforge-ffi-stream`（事件流积木）**：Rust Channel ↔ Dart `Stream` 双向绑定，用于进度推送、实时订阅。
- 📦 **`archforge-observe-telemetry`（全链路观测积木）**：跨语言全链路追踪，支持把客户端 Trace 周期性上报 Sentry / Jaeger。

---

## 7. 核心实现机制

### 7.1 Ports & Adapters（核心实现方式）

**Domain 定义 Trait（Port）**，Adapters 提供 `impl`。

**严格规范**：

- Port 置于 `domain-core` 或对应 Bounded Context。
- Adapter 置于独立 `adapters/` crate。
- 使用 `async_trait` 支持异步。
- Adapter 层将 InfraError 映射为 DomainError。

**第一步：制造插槽（Domain 层纯净定义）**

```rust
// business_blocks/auth-domain/src/ports.rs
use crate::entities::User;
use arch_core_error::AppError;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: &User) -> Result<(), AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
}
```

**第二步：制造积木（Infra 层适配实现）**

```rust
// infra_adapters/infra-seaorm-sqlite/src/user_repo.rs
use auth_domain::ports::UserRepository;
use sea_orm::DatabaseConnection;

pub struct SqliteUserRepository {
    pub db: DatabaseConnection,
}

#[async_trait::async_trait]
impl UserRepository for SqliteUserRepository {
    async fn save(&self, user: &User) -> Result<(), AppError> {
        // 数据库写入逻辑
    }
    // ...
}
```

### 7.2 Domain 建模规范（工业级 DDD）

- **Value Object**：不可变、值相等、自校验（`new()` 返回 `Result`）。
- **Entity**：唯一标识（`Uuid` / Newtype）、字段私有、工厂方法（Always Valid）。
- **Aggregate**：一致性边界 + Root + Domain Events。
- **Domain Event**：`enum` + Payload，支持 Outbox。
- **Rich vs Anaemic**：复杂业务优先 Rich Domain Model。

### 7.3 Application Layer

Use Case 作为入口：

- 输入：Command / Query（带验证）。
- 输出：Response DTO。
- 职责：编排、事务、权限、事件发布、日志。

**CQRS 支持**：可选 feature，提供简单 Bus 或 Handler 注册表。

### 7.4 Infrastructure

- 所有外部交互通过 Port 抽象。
- 支持多 DB、多缓存、多消息队列共存。

### 7.5 一键拼装：Cargo Features 与依赖注入

**第三步：Feature 开关**

在 `bridge_api/Cargo.toml` 中通过 feature 决定使用哪套基础设施：

```toml
[dependencies]
auth-application = { path = "../business_blocks/auth-application" }

infra-mock-memory   = { path = "../infra_adapters/infra-mock-memory",   optional = true }
infra-seaorm-sqlite = { path = "../infra_adapters/infra-seaorm-sqlite", optional = true }

[features]
default        = ["sqlite-backend"]
mock-backend   = ["dep:infra-mock-memory"]
sqlite-backend = ["dep:infra-seaorm-sqlite"]
```

**第四步：装配车间（Dependency Injection Setup）**

```rust
// bridge_api/src/setup.rs
use std::sync::Arc;
use auth_application::AuthAppService;

pub struct AppState {
    pub auth_service: AuthAppService,
}

pub async fn init_app_state() -> Arc<AppState> {
    #[cfg(feature = "sqlite-backend")]
    let user_repo = Arc::new(infra_seaorm_sqlite::SqliteUserRepository::new().await);

    #[cfg(feature = "mock-backend")]
    let user_repo = Arc::new(infra_mock_memory::MockUserRepository::new());

    let auth_service = AuthAppService::new(user_repo);

    Arc::new(AppState { auth_service })
}
```

---

## 8. Flutter 集成与跨平台策略

**分层推荐**（AppFlowy 风格）：

- **Rust**：Domain + Application + 重型 Infra（计算、持久化、安全）。
- **Flutter**：Presentation + State Management（Riverpod / Bloc）+ 轻量 DTO。

**Bridge 最佳实践**：

- 使用 `flutter_rust_bridge` v2 + `#[frb]` 暴露**高阶 Use Case API**。
- 避免大对象传输，优先序列化 DTO。
- Flutter 侧采用 Feature-First + Clean 分层。

### 8.1 Rust 暴露高阶用例

```rust
// bridge_api/src/api.rs
use flutter_rust_bridge::frb;
use auth_application::commands::LoginCommand;

#[frb(sync = false)]
pub async fn api_login(cmd: LoginCommand) -> anyhow::Result<LoginResponse> {
    let state = get_app_state();
    let result = state.auth_service.execute_login(cmd).await?;
    Ok(LoginResponse::from(result))
}
```

### 8.2 Flutter 端 Riverpod 承接

```dart
// flutter_app/lib/features/auth/providers/auth_provider.dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:myapp/core/bridge/api.dart'; // Rust 生成的 binding

final authProvider = AsyncNotifierProvider<AuthNotifier, UserState>(() => AuthNotifier());

class AuthNotifier extends AsyncNotifier<UserState> {
  @override
  Future<UserState> build() async => const UserState.unauthenticated();

  Future<void> login(String email, String password) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      final response = await apiLogin(
        cmd: LoginCommand(email: email, password: password),
      );
      return UserState.authenticated(response.token);
    });
  }
}
```

---

## 9. 拼装蓝图 (Architecture Blueprints)

通过 `Cargo.toml` 按需引入（feature 或独立 crate），即可组合出截然不同的架构流。

### 9.1 蓝图 A：极简记事本 App（Startup MVP）

- **需求**：两周内上线跨平台记事本，附带简单本地存储。
- **拼装清单**：
  - Domain：`archforge-domain-lite`
  - App：`archforge-app-service`
  - Infra：`archforge-infra-sqlite-fast`
  - Boundary：`archforge-ffi-bridge`
- **架构流**：Flutter UI → FFI 桥 → `NoteService` → SQLite Adapter → DB。一条线直通，毫无冗余。

### 9.2 蓝图 B：离线优先的重型效率工具（AppFlowy 类）

- **需求**：复杂树形节点、频繁读写、丝滑 UI、未来支持协同。
- **拼装清单**：
  - Domain：`archforge-domain-rich`
  - App：`archforge-app-cqrs`（打字 = Command，UI 树渲染走 Query）
  - Infra：`archforge-infra-seaorm` + `archforge-infra-kv`（草稿缓存）
  - Boundary：`archforge-ffi-stream`（Rust → Flutter Riverpod 实时推送）
- **架构流**：UI 触发 Command → Rust CQRS 写入 Domain 并落盘 → Stream 发事件 → Flutter Query 自动刷新。

### 9.3 蓝图 C：金融级加密资产钱包

- **需求**：安全极高、复杂密码学、绝不允许非法状态、完整审计。
- **拼装清单**：
  - Domain：`archforge-domain-typestate`（转账：创建 → 签名 → 广播，严格状态机）
  - App：`archforge-app-workflow`（本地与链上状态最终一致）
  - Observe：`archforge-observe-telemetry`（记录每一步耗时与潜在 Panic）

### 9.4 开发者体验 (DX)

开发者在初始化项目时，看到的 `Cargo.toml` 应该是这样：

```toml
[dependencies]
archforge = { version = "1.0", features = [
    "domain-rich",    # DDD 的 Entity / 聚合根宏
    "app-cqrs",       # 命令 / 查询总线
    "infra-sqlite",   # 本地 SQLite
    "ffi-frb-stream", # Flutter 数据流
    "observe-tracing" # 日志追踪
]}
```

在 Rust 端，组装入口同样是“搭积木”：

```rust
fn init_app() -> AppContainer {
    // 1. 挂载基础设施积木
    let db = SqliteAdapter::new("app.db");

    // 2. 挂载命令总线积木
    let mut command_bus = CqrsBus::new();

    // 3. 注入具体业务领域
    command_bus.register_handler::<CreateUserCommand, _>(CreateUserHandler::new(db.clone()));

    // 4. 打包返回给 FFI 层
    AppContainer::build()
        .with_bus(command_bus)
        .with_telemetry(TracingConfig::default())
        .finish()
}
```

---

## 10. 工程流程与治理 (Processes & Governance)

- **项目初始化**：`cargo generate` + 复杂度选择。
- **新 Bounded Context**：`just new-context <name>`。
- **测试金字塔**：Unit（Domain）→ Integration（Memory）→ Component（testcontainers）→ E2E。
- **CI/CD**：GitHub Actions / GitLab CI（lint、test、coverage、security、benchmark）。
- **版本管理**：Semantic Versioning 2.0（Trait 变更 = Major）。
- **决策**：所有架构变更必须写 ADR。
- **贡献**：PR Template + Architecture Impact Review。

### 10.1 新需求标准研发 SOP

以新增「任务管理」模块为例：

1. **Rust 领域建模**：在 `task-domain` 定义 `Task` Entity、状态机以及 `TaskRepository` Trait（极速测试：纯 Rust 单测，零外部依赖）。
2. **Rust 业务编排**：在 `task-application` 编写 `CreateTaskUseCase`。
3. **Mock 注入**：用 `infra-mock-memory` 快速实现 Repo 插槽，并把 API 暴露给 Flutter。
4. **Flutter UI 并行开发**：前端直接调用 Rust API，数据走内存 Mock，无需等待建表。
5. **真实落地**：编写 `infra-seaorm-sqlite` 实现真正的落盘逻辑。
6. **无缝切换**：把 `Cargo.toml` 的 feature 切到 `sqlite-backend`，重新编译——业务代码与 UI 代码零修改即可上线。

### 10.2 脚手架 (CLI / cargo-generate)

```bash
cargo generate archforge/template --name my_app
```

交互式选项示例：

1. `Which Database block? [Sqlite, Postgres, None]` → 选 Sqlite。
2. `Enable CQRS block? [y/N]` → 选 y。
3. `Enable Flutter Stream bindings? [y/N]` → 选 y。

执行完即可得到：前后端打通、物理分层清晰、自带编译期依赖注入、Riverpod 模板齐全的混合工程模板。

---

## 11. 测试、可观测性、安全、性能规范

- **测试矩阵**：详见 `docs/testing-strategy.md`。
- **Observability**：`tracing` + OpenTelemetry + Prometheus + Jaeger。
- **安全**：OWASP Top 10、输入验证（`validator`）、Secret 管理、权限位于 Application 层。
- **性能**：关键路径 Benchmark、零拷贝优先；高性能场景可选 Data-Oriented 模块。

---

## 12. 风险分析与缓解 (Risks & Mitigations)

- **过度抽象导致 boilerplate** → 缓解：提供 Minimal 模板 + Macros。
- **Flutter Bridge 性能开销** → 缓解：高阶 API + 基准测试。
- **维护负担重** → 缓解：从真实项目提取 + 社区驱动 + 清晰边界。

---

## 13. 路线图与里程碑

- **Phase 1（MVP — 2026 Q2）**：`domain-core` + memory adapter + User 示例 + Flutter Bridge。
- **Phase 2（Core — 2026 Q3）**：主流 Adapters + CQRS + Event Sourcing + Macros。
- **Phase 3（Enterprise — 2026 Q4+）**：CLI、Saga、多租户、可视化工具、更多 Flutter 集成。

---

## 14. 参考文献与致谢

- Eric Evans — *Domain-Driven Design*。
- Robert C. Martin — *Clean Architecture*。
- Vaughn Vernon — *Implementing Domain-Driven Design*。
- 社区项目：rust-ddd-skeleton、clean-architecture-rust、AppFlowy、eldimious/rust-api-ddd 等。
- 方法论：C4 Model、ADR 实践。

---

**文档结束。**
