# Engineering Exceptions

本文件记录当前项目相对 [Flutter_Rust工程手册.md](Flutter_Rust工程手册.md) 的有意识偏离。每一项都必须有风险、退出条件和优先级，避免临时方案变成默认架构。

## E-001 Dart/sqflite 暂作业务事实源

- 状态: 已存在
- 位置: [lib/data/database.dart](lib/data/database.dart), [lib/state/fragments_provider.dart](lib/state/fragments_provider.dart)
- 原因: MVP 阶段优先验证记录、恢复、淡化体验；Rust repository 尚未落地。
- 风险: 业务规则进入 Provider；迁移、事务、查询和隐私策略分散在 Dart；后续 Rust 化成本增加。
- 退出条件: Rust 提供 `create_fragment`、`record_recovery`、`list_fragments`、`get_fragment_detail` 等 use case API；Dart 只保留 view state。
- 优先级: P1

## E-002 本地情绪数据暂未加密

- 状态: 已存在
- 位置: [lib/data/database.dart](lib/data/database.dart)
- 原因: MVP 阶段先使用 sqflite 明文库验证数据模型。
- 风险: 情绪内容、恢复记录、标签和强度属于高敏本地数据，设备被访问或备份外泄时存在隐私风险。
- 退出条件: 引入 SQLCipher、文件级加密或 Rust 侧加密仓储；补充导出、删除、迁移和恢复测试。
- 优先级: P0

## E-003 草稿暂存于 SharedPreferences

- 状态: 已存在
- 位置: [lib/services/app_settings.dart](lib/services/app_settings.dart)
- 原因: 保护用户输入过程，避免误退出导致内容丢失。
- 风险: 草稿可能包含完整私密文本，SharedPreferences 不适合作为敏感文本长期存储。
- 退出条件: 改为加密草稿仓储，或仅保留内存草稿并给用户显式保存入口。
- 优先级: P0

## E-004 Provider 作为当前状态框架

- 状态: 已存在
- 位置: [lib/app.dart](lib/app.dart), [lib/state/fragments_provider.dart](lib/state/fragments_provider.dart)
- 原因: 当前页面数量少，Provider 能快速支撑 MVP。
- 风险: ChangeNotifier 容易吸收业务逻辑并造成大范围 rebuild。
- 退出条件: 业务逻辑迁至 Rust application；Provider 仅做 view state，或通过 ADR 决定迁移到 Riverpod。
- 优先级: P2

## E-005 flutter_rust_bridge demo API 已清理

- 状态: 已解决（2026-05）
- 位置: [rust/src/api/simple.rs](rust/src/api/simple.rs)
- 处理: `greet` 演示函数已删除，`simple.rs` 仅保留 `init_app()` FRB 初始化入口。
- 后续: 若新增 demo / 调试用 FFI，统一放到 `rust/src/api/__dev/` 或 feature gate 后再开放。
