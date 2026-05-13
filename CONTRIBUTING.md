# 贡献指南

感谢有兴趣参与 SpendWhy。本文档汇总参与开发前需要了解的工程约束、本地环境与提交规范。详细的架构原则请阅读 [Flutter_Rust工程手册.md](Flutter_Rust工程手册.md)。

## 1. 行为规范

请保持友善、尊重隐私、聚焦技术讨论。SpendWhy 处理的是用户的情绪记录，对数据安全与隐私保持高敏感度。

## 2. 本地开发环境

| 工具 | 版本 | 说明 |
|---|---|---|
| Flutter | 与 `pubspec.yaml` 中 `environment.sdk` 一致 | 推荐用 `fvm` 管理 |
| Rust | 见 `rust-toolchain.toml` | 自动安装 |
| Android SDK | API 34+ | 含 NDK r26 |
| Xcode | 15+（仅 macOS） | iOS 构建 |
| flutter_rust_bridge_codegen | 2.12.0 | `cargo install` 安装 |

Windows 开发者首次拉取后请运行：

```powershell
.\doctor.ps1
```

## 3. 分支与提交

- 主分支：`main`。日常开发请基于 feature 分支：`feat/xxx`、`fix/xxx`、`docs/xxx`、`refactor/xxx`。
- 提交信息采用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/v1.0.0/)：
  - `feat: 新增碎片标签筛选`
  - `fix(rust): 修复 fade 边界条件`
  - `docs: 更新 ADR-0002`
  - `chore: 升级 flutter_rust_bridge`
- 一个 PR 解决一件事，避免混合无关变更。

## 4. 代码风格

- Dart：`flutter analyze` 必须 0 issue；`dart format` 必须无 diff；遵守 [analysis_options.yaml](analysis_options.yaml) 中的严格模式与 lint 规则。
- Rust：`cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings` 必须通过。
- 编辑器请尊重 [.editorconfig](.editorconfig) 与 [.gitattributes](.gitattributes)（默认 LF、UTF-8、空格缩进）。
- 不要提交 `lib/src/rust/` 与 `rust/src/frb_generated.rs` 的手改；若需重生成请运行：

  ```powershell
  flutter_rust_bridge_codegen generate
  ```

## 5. 测试要求

提交前请至少跑过：

```powershell
flutter pub get
flutter analyze
dart format --output=none --set-exit-if-changed .
flutter test

Push-Location rust
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
Pop-Location
```

涉及 FFI 边界或新增 API 时，需要补充：
- Rust 端单元测试或 property test；
- Dart 端 mock 测试，覆盖 DTO 转换与错误路径；
- 必要时补充 `integration_test/` 端到端用例。

## 6. 架构约束

- 业务事实源的长期归属是 Rust，详见 [docs/adr/0001-state-and-data-boundary.md](docs/adr/0001-state-and-data-boundary.md)。
- 任何对手册（[Flutter_Rust工程手册.md](Flutter_Rust工程手册.md)）的有意识偏离，必须同步在 [EXCEPTIONS.md](EXCEPTIONS.md) 中登记风险、退出条件与优先级。
- 新增重大决策请创建 ADR：`docs/adr/NNNN-title.md`。

## 7. 隐私与安全

- 严禁在示例、测试、日志中包含真实用户数据。
- 触及加密、密钥、备份、导出能力时，请阅读 [SECURITY.md](SECURITY.md) 并在 PR 描述中说明影响面。

## 8. 提交 PR 清单

- [ ] CI 全部绿
- [ ] 新增/修改公共 API 已更新文档
- [ ] 触及架构边界的变更已记录 ADR 或 EXCEPTIONS
- [ ] `CHANGELOG.md`（若存在）的 `[Unreleased]` 已更新
