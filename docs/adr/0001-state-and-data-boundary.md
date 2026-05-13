# ADR 0001: 当前状态框架与数据边界

## 状态

Accepted for MVP, revisit before M2.

## 背景

SpendWhy 当前以 Flutter 快速验证体验为主，已经接入 Rust 计算引擎，但数据事实源仍在 Dart/sqflite。工程手册的长期目标是 Rust 承担 domain、application、repository 和关键隐私策略，Flutter 只保留 presentation 与 view state。

## 决策

MVP 阶段继续使用 Provider/ChangeNotifier 管理页面状态，继续使用 Dart/sqflite 保存 fragments 与 recoveries。Rust 当前只承载淡化和和解分数计算。

## 约束

- Provider 不再新增复杂业务规则。
- 新的跨页面业务用例优先设计为 Rust application API。
- 新增敏感数据字段时，必须同步更新 [EXCEPTIONS.md](../../EXCEPTIONS.md) 或迁入加密仓储。
- 任何新增列表查询必须优先考虑分页或局部更新。

## 退出计划

进入 M2 前，将 `create_fragment`、`record_recovery`、`list_fragments`、`get_fragment_detail` 迁到 Rust use case。Flutter Provider 只保存加载状态、错误状态和当前页面 view model。
