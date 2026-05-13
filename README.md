# SpendWhy / fragments

SpendWhy 是一个 Flutter + Rust 的情绪记录与恢复陪伴应用。Flutter 负责界面、交互和轻量视图状态，Rust 负责可复用的情绪淡化与和解分数计算。项目当前处于 MVP 到工程化基础阶段，目标是逐步对齐 [Flutter_Rust工程手册.md](Flutter_Rust工程手册.md)。

## 当前架构

- Flutter 入口: [lib/main.dart](lib/main.dart)
- App 组装与 Provider: [lib/app.dart](lib/app.dart)
- 视图状态: [lib/state/fragments_provider.dart](lib/state/fragments_provider.dart)
- Dart 本地数据层: [lib/data/database.dart](lib/data/database.dart)
- Rust 桥接门面: [lib/services/rust_backend.dart](lib/services/rust_backend.dart)
- Rust 计算逻辑: [rust/src/api/fade.rs](rust/src/api/fade.rs)
- FRB 配置: [flutter_rust_bridge.yaml](flutter_rust_bridge.yaml)

## 开发命令

```powershell
flutter pub get
flutter analyze
flutter test
Push-Location rust
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
Pop-Location
```

Windows + Android + Rust 构建前建议运行：

```powershell
.\doctor.ps1
```

## 工程状态

当前 Rust 已接入 Flutter，并承担 `fade` 与 `growth_score` 计算。业务事实源仍在 Dart/sqflite，隐私数据加密、Rust repository、分页、事件流、真实 FFI 集成测试等能力仍在推进中。已知例外和退出计划记录在 [EXCEPTIONS.md](EXCEPTIONS.md)。

## 发布注意事项

Android release 构建不再使用 debug signing。发布前需要在 `android/key.properties` 中配置正式签名信息，或者通过 CI secrets 注入同等配置。
