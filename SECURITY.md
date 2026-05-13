# 安全策略

## 支持版本

| 版本 | 状态 |
|---|---|
| `main` (开发分支) | 接收安全修复 |
| `0.1.x` (MVP) | 接收安全修复 |
| 更早 | 不再维护 |

## 报告漏洞

请勿在公开 Issue 中披露安全问题。请通过以下任一渠道私下提交：

- GitHub Security Advisory：https://github.com/jusu2/spendwhy/security/advisories/new
- 邮件：在仓库 `README.md` 或维护者 GitHub Profile 中获取联系方式

我们会在 3 个工作日内确认收悉，并在评估后给出修复或缓解时间表。

## 当前已知风险

下列条目已在 [EXCEPTIONS.md](EXCEPTIONS.md) 中跟踪并具备退出计划：

- **E-002** 本地情绪数据未加密（SQLite 明文存储）— 优先级 P0
- **E-003** 草稿暂存于 `SharedPreferences` — 优先级 P0

在退出条件完成前，请注意：
- 不要在共享或多用户设备上使用 Debug 版本。
- 设备备份（iCloud / Google Backup）可能包含未加密的情绪数据，请按个人风险偏好关闭对应应用的备份。

## 数据范围与处理原则

- SpendWhy 不会主动上传任何用户输入的情绪文本、标签或恢复记录。
- Rust 计算层是纯函数式淡化/和解分数计算，不持久化输入。
- 所有崩溃日志、telemetry 在引入前必须经过 ADR 评审，且默认关闭。

## 加密路线

- M2 之前：迁移至 SQLCipher 或 Rust 侧加密仓储；草稿改为 `flutter_secure_storage` 或显式保存。
- 长期：Rust infra 层统一密钥管理，密钥由 OS Keychain / Keystore 派生，禁止常驻内存明文。

## 依赖与供应链

- Rust 依赖在 [rust/Cargo.toml](rust/Cargo.toml) 锁定主版本；通过 `cargo deny` / Dependabot 检查漏洞（CI 待补）。
- Dart 依赖通过 `flutter pub outdated --mode=null-safety` 周期性审视。
- 任何引入新依赖的 PR 必须说明用途、许可证以及替代方案评估。
