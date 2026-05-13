# ArchForge MVP

Rust + Flutter 工业级架构积木库。**这是 Step 0–2 的可运行 MVP**：kernel + 一致性测试 harness + 一个完整的 `auth` 垂直切片（DTO/Domain/App/双 Adapter）。所有代码可编译、可测试、可运行；其余 crate 见 [`ROADMAP.md`](ROADMAP.md)。

## 设计宪法

必读：[`ARCHITECTURE_INVARIANTS.md`](ARCHITECTURE_INVARIANTS.md)。**5 条不可动摇的不变量** 是整个体系存在的根基。

## 工作区结构

```text
archforge/
├── Cargo.toml                  # workspace 根
├── ARCHITECTURE_INVARIANTS.md  # 立宪
├── ROADMAP.md                  # 还未实现的 crate 与下一步
├── deny.toml                   # 依赖白名单（cargo-deny）
├── kernel/                     # 不可动摇的内核（< 500 行，1.0 后冻结）
├── conformance/                # Port 一致性测试 harness
├── contract-auth/              # 跨层契约：DTO / Cmd / Query / Event / Error / Port
├── domain-auth/                # 富领域模型（Always-Valid）
├── app-auth/                   # 用例编排
├── infra-auth-memory/          # DashMap 后端
├── infra-auth-jsonfile/        # 异步文件后端（零原生依赖、跨平台）
└── examples/auth-cli/          # 端到端装配演示（Cargo feature 切换 backend）
```

## 一分钟跑通

```powershell
cd archforge
cargo test --workspace
cargo run -p auth-cli --features memory-backend -- demo
cargo run -p auth-cli --features jsonfile-backend -- demo
```

业务代码 + 用例代码在两个 backend 之间**零修改**——这是积木库正确性的最直接证明。

## 关键工程化亮点

1. **同一组 conformance property test 跑遍所有 Adapter**——LSP 的工程化兑现，见 `conformance/src/user_repo.rs`。任何新 Adapter 必须把自己塞进这组测试通过。
2. **Cargo feature 切换 backend，业务代码零修改**——见 `examples/auth-cli/Cargo.toml` 的 `[features]`。
3. **Port 错误是 narrow enum + `#[non_exhaustive]`**，底层错误（io、serde）在 Adapter 内部收敛，不泄漏到 contract 层。
4. **Newtype 在 contract 层一次校验**——`Email`、`UserId` 等通过 `arch_newtype!` 宏获得自动 `serde` 校验，进入系统第一行即合法。
5. **Domain 不离开自己的 crate**——所有跨层调用走 DTO，`User` 富模型仅在 `domain-auth` 内可见。

## 下一步

阅读 [`ROADMAP.md`](ROADMAP.md)。简而言之：

- Step 3：抽 `archforge-app-cqrs`、`archforge-domain-typestate`、UoW、Outbox。
- Step 4：`cargo-deny` / `cargo-public-api` / `cargo-semver-checks` / mutation testing 全套护城河。
- Step 5：`cargo-generate` 模板与 CLI。
- Step 6：兼容性矩阵 CI、社区 RFC 流程。
