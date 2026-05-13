# Flutter + Rust App 工程手册

版本: v4.7
日期: 2026-05-11
定位: 独立开发者与小型高标准团队的 Flutter + Rust App 架构宪法、工程手册与长期路线图。
适用平台: 默认 iOS / Android；Web / Desktop 仅在产品明确需要时扩展。
合并来源: 《白皮书.md》与《开发哲学.md》。本文为统一版，整合长期架构愿景、日常开发纪律、质量门禁、发布准则与 SpendWhy 当前落地路线。
约定: 本手册默认条款是"硬规则"。任何偏离必须有 ADR 或 `EXCEPTIONS.md` 记录；术语首次出现优先使用全称，常用缩写见附录 A 术语表。

---

## 0. 摘要

Flutter + Rust 的价值，不是为了混合技术栈而混合，而是把两种能力放到各自最适合的位置:

1. Flutter 负责体验: 页面、交互、动效、可访问性、平台一致性。
2. Rust 负责内核: 业务规则、状态机、持久化、加密、搜索、同步、批量计算、性能关键路径。
3. flutter_rust_bridge 负责边界: 类型生成、异步调用、事件流、大数据通道。

这套组合的收益是可靠的核心逻辑、快速的跨平台体验、稳定的用例式 FFI 契约，以及可测量的性能和稳定性。它的代价是工具链复杂、热重载边界变重、构建和 CI 成本上升、架构纪律要求更高。

本文档回答五个问题:

1. 什么时候该用 Flutter + Rust。
2. Flutter、Dart Application、FFI、Rust Domain、Rust Infra 各自应该做什么。
3. 代码、数据、状态、错误、性能、安全、发布要达到什么标准。
4. 高级架构能力何时启用，何时必须停下。
5. SpendWhy / 碎片当前应如何逐步落地。

### 0.1 如何使用本手册（按角色检索）

本手册是工具书，不要试图一次读完。按当前任务进入对应章节：

| 我现在要做 | 必读 | 选读 |
|------------|------|------|
| 加入项目第一天 | §0.1, §1, §2, §5, 附录 A 术语表, §18.7 Onboarding | §28 SpendWhy 当前路线 |
| 设计一个新 FFI API | §6 全章, §7.1 DTO 戒律, §11.2 错误契约, §15.3 Trace 透传, §24.3 FFI 检查清单 | §6.7 演进与版本协议 |
| 设计一个新功能 / 用例 | §2.1 思维模型, §4.1 决策树, §8 状态一致性, §17.4 UX 状态机, §24.2 功能完成清单 | §9 离线同步 |
| 写 Rust 业务代码 | §5.5 Rust 内部分层, §11 错误, §10 并发取消, §16 测试 | §29 极端架构 |
| 写 Dart UI / Provider | §5, §8 Riverpod/状态, §17 i18n/a11y/UX, §11.3 错误到 UI 映射 | §13 性能预算 |
| 排查性能问题 | §13 全章, §15 可观测性, §27 判例库 | §29 极端架构 |
| 排查崩溃 / 内存 | §11.5 panic, §12 大对象, §15 观测, §27 判例库 | §6.5.5 RustOpaque |
| 升级依赖 / 工具链 | §6.5.1 FRB 版本治理, §19.5 兼容矩阵, §18.5 CI | §18.10 doctor |
| 准备发版 | §13.4 SLO, §14 安全隐私, §17.6/17.7/17.8 三大检查清单, §20 发布回滚, §24.4 发布前清单 | §20.3 灾备 |
| 处理事故 | §14.6 安全事件响应, §18.8 事故复盘, §27 判例库 | §11 错误 |
| 修改 / 维护本手册 | §30 文档治理, §30.1 演进证据, §24.5 模板/CI 变更清单 | §31 最终原则 |

阅读规则：

1. 不要把本手册当 README 直接执行；它是判断标准，具体命令在 [README.md](README.md) 与 [doctor.ps1](doctor.ps1)。
2. 当某条规则与现实冲突时，按 §30 走 ADR / EXCEPTIONS / 更新手册，不要默默偏离。
3. 章节之间通过编号交叉引用；缩写在附录 A 术语表查询。

### 0.2 目录

- [0. 摘要](#0-摘要)
  - [0.1 如何使用本手册](#01-如何使用本手册按角色检索)
  - [0.2 目录](#02-目录)
- [1. 使用方式](#1-使用方式)
- [2. 顶层原则](#2-顶层原则)
  - [2.1 核心思维模型](#21-核心思维模型)
- [3. 适用边界](#3-适用边界)
- [4. 架构成熟度模型](#4-架构成熟度模型)
  - [4.1 核心决策树](#41-核心决策树)
  - [4.2 架构取舍矩阵](#42-架构取舍矩阵)
- [5. 总体分层](#5-总体分层)
  - [5.5 Rust 内部分层规范](#55-rust-内部分层规范)
- [6. FFI 设计标准](#6-ffi-设计标准)
  - [6.1 四档调用模型](#61-四档调用模型)
  - [6.2 用例式 API](#62-用例式-api)
  - [6.5 flutter_rust_bridge 工程细节](#65-flutter_rust_bridge-工程细节)
  - [6.7 FRB 代码生成工作流（操作篇）](#67-frb-代码生成工作流操作篇)
  - [6.8 FRB 常用代码模式](#68-frb-常用代码模式)
  - [6.9 FFI 演进与版本协议](#69-ffi-演进与版本协议)
  - [6.10 Android 平台构建细节](#610-android-平台构建细节)
  - [6.11 iOS 平台构建细节](#611-ios-平台构建细节)
  - [6.12 Platform Channels 与 FRB 共存指引](#612-platform-channels-与-frb-共存指引)
  - [6.13 Flutter ↔ Rust 调用模式全景](#613-flutter--rust-调用模式全景)
- [7. 数据建模与持久化](#7-数据建模与持久化)
- [8. 状态一致性与用户体验](#8-状态一致性与用户体验)
- [9. 离线优先与同步](#9-离线优先与同步)
- [10. 并发、取消与背压](#10-并发取消与背压)
- [11. 错误、崩溃与恢复](#11-错误崩溃与恢复)
  - [11.6 Unsafe Rust 边界](#116-unsafe-rust-边界)
- [12. 内存与大对象](#12-内存与大对象)
- [13. 冷启动与性能工程](#13-冷启动与性能工程)
- [14. 安全、隐私与合规](#14-安全隐私与合规)
- [15. 可观测性](#15-可观测性)
  - [15.5 Tracing 桥接实操](#155-tracing-桥接实操)
- [16. 测试体系](#16-测试体系)
  - [16.4 FFI Mock 与集成测试范式](#164-ffi-mock-与集成测试范式)
- [17. 产品质量: i18n、a11y 与设计系统](#17-产品质量-i18na11y-与设计系统)
- [18. 工程效能与仓库治理](#18-工程效能与仓库治理)
- [19. 依赖治理](#19-依赖治理)
- [20. 发布、回滚、灾备与退役](#20-发布回滚灾备与退役)
- [21. 法律、授权与资源](#21-法律授权与资源)
- [22. AI 辅助开发协议](#22-ai-辅助开发协议)
- [23. 反模式黑名单](#23-反模式黑名单)
- [24. 自检清单](#24-自检清单)
- [25. 技术债务台账模板](#25-技术债务台账模板)
- [26. ADR 模板](#26-adr-模板)
- [27. 判例库](#27-判例库)
- [28. SpendWhy / 碎片当前落地路线](#28-spendwhy--碎片当前落地路线)
- [29. 极端架构附录](#29-极端架构附录)
- [30. 文档治理](#30-文档治理)
- [31. 最终原则](#31-最终原则)
- [附录 A. 术语表](#附录-a-术语表)
- [变更记录](#变更记录)

---

## 1. 使用方式

这份文档是默认执行标准，不是灵感集合。任何工程决策先回答三个问题:

1. 该不该做。
2. 放在哪一层做。
3. 做到什么标准才算完成。

允许例外，但例外必须写入 `EXCEPTIONS.md`，包含理由、风险、补偿措施、失效日期和迁移计划。没有记录的例外，就是架构漂移。

例外模板:

```markdown
## EX-0001: Dart 侧临时缓存草稿

- 日期: 2026-05-03
- 违反条款: 业务主数据默认由 Rust 持有
- 原因: 草稿仅用于 UI 输入恢复，不参与跨端同步和业务查询
- 风险: 与 Rust 数据源形成双写错觉
- 补偿: 命名为 UiDraftCache，禁止 feature 外访问
- 失效日期: 2026-06-01
- 迁移计划: 表单稳定后改为 Rust draft repository
```

---

## 2. 顶层原则

独立开发者真正的护城河不是会多少技术，而是能否用稳定标准持续交付高质量产品。

默认 App 标准:

1. 小而强，不臃肿。
2. 快而稳，不玄学。
3. 私密安全，不透支用户信任。
4. 离线可用，不把网络当核心体验的前置条件。
5. 可观测、可恢复、可回滚，不靠祈祷上线。
6. 架构长期一致，不每个项目重新发明自己。

九条铁律:

1. Rust 是业务内核，Flutter 是体验层。
2. FFI 必须粗粒度，一次调用承载一个完整用例。
3. UI 线程不可被重活污染，超过一帧预算的任务必须离开 UI 路径。
4. 数据必须可批量、可分页、可增量、可观测。
5. 跨边界数据优先扁平化、列式化、连续内存化。
6. 状态流必须单向: User Action -> Command -> Rust -> Event -> State -> UI。
7. 安全和隐私是产品能力，不是上线前补丁。
8. 没有指标就没有优化。
9. 简单优于聪明，纪律高于灵感。

冲突时按以下顺序裁决:

1. 正确性。
2. 用户数据安全。
3. 隐私合规。
4. 可观测性。
5. 性能。
6. 可维护性。
7. 开发速度。
8. 新颖性。

### 2.1 核心思维模型

这份手册不是规则堆叠，而是一套看待 App 的系统模型。所有工程判断都围绕五个问题展开: 事实在哪里、用例是什么、边界在哪里、失败如何恢复、复杂度是否值得。

#### 2.1.1 事实源模型

```text
Fact Source -> Read Model -> DTO -> View State -> Widget
     ^                                      |
     |                                      v
  Command <- Dart Controller <- User Action
```

1. Rust repository 是业务事实源，决定什么是真的。
2. Read model 是为了查询和 UI 效率存在的投影，可以重建。
3. DTO 是跨 FFI 的契约快照，不是内部实体。
4. Flutter state 是当前页面需要的视图状态，不是业务真相。
5. Widget 只表达状态，不决定事实。

判断一句代码该放哪里，先问它是否改变事实源。如果会改变事实源，它必须进入 Command / UseCase / Rust mutation 路径；如果只改变交互反馈，它可以留在 Flutter。

#### 2.1.2 用例模型

用户不是在调用函数，而是在发起用例。一个用例必须回答:

1. 谁发起。
2. 作用于什么对象。
3. 需要什么权限和前置状态。
4. 成功后产生什么事实变化。
5. 失败后用户如何理解和恢复。
6. 需要产生什么事件、指标和日志。

因此 FFI API 不为 Widget 服务，而为系统用例服务。`load_fragment_detail` 是用例，`get_fragment_title` 是跨语言 getter。

#### 2.1.3 一致性窗口模型

移动端体验不是永远强一致，而是管理可解释的一致性窗口。

| 场景 | 允许窗口 | 纠正机制 |
|------|----------|----------|
| UI 输入草稿 | 页面生命周期 | 用户提交或页面销毁 |
| 乐观更新 | action pending 期间 | Rust ActionResult / rollback |
| 离线写入 | 同步完成前 | sync state / conflict state |
| Read model 缓存 | 缓存 TTL 或事件到达前 | Rust event / refresh |
| 服务端裁决 | 请求完成前 | server response / retry / conflict |

规则: 可以短暂不一致，但必须能命名、能观测、能纠正、能向用户解释。

#### 2.1.4 复杂度预算模型

每个功能都消耗复杂度预算。高级技术不是奖励，而是贷款。

1. Actor 购买生命周期隔离，代价是消息协议和调试复杂度。
2. CRDT 购买离线自动合并，代价是 op log、GC、迁移和冲突可视化。
3. Snapshot + WAL 购买启动速度，代价是格式演进和崩溃恢复。
4. OpenTelemetry 购买跨层定位，代价是采样、成本和噪音治理。
5. ECS 裸绘购买海量节点性能，代价是放弃大量 Flutter Widget 生态能力。

引入复杂度前必须回答三个问题:

1. 当前瓶颈是否被测量证明。
2. 简单方案是否已经达到边界。
3. 退出或降级路径是否清楚。

#### 2.1.5 失败优先模型

顶级 App 不是不会失败，而是失败时不丢数据、不误导用户、不让开发者失明。

每个核心功能设计时先写失败路径:

1. 输入非法怎么办。
2. Rust 返回业务错误怎么办。
3. 网络断开怎么办。
4. 本地 DB 损坏怎么办。
5. 页面退出时任务还在跑怎么办。
6. 用户重复点击怎么办。
7. 事故发生后如何定位 trace。

如果失败路径说不清，成功路径暂时不进入实现。

---

## 3. 适用边界

### 3.1 适合使用 Flutter + Rust 的场景

1. App 有明显端上业务内核，例如复杂状态机、评分规则、推荐逻辑、离线策略。
2. App 需要长期离线可用，本地数据不是缓存，而是主工作区。
3. App 有较高隐私要求，希望核心数据处理尽量在端上完成。
4. App 需要高性能本地搜索、批处理、加密、压缩、导入导出。
5. 同一套业务内核未来可能复用到多个端或多个产品。
6. 开发者愿意长期维护 Rust 工具链，并把架构纪律写入流程。

### 3.2 不适合使用 Flutter + Rust 的场景

1. 纯 UI 原型或纯 CRUD MVP，业务是否成立尚未验证。
2. App 的核心逻辑几乎全在服务端，端上只是展示。
3. 项目生命周期很短，不值得承担双栈维护成本。
4. 需要深度原生系统能力，例如复杂相机、AR、低层后台服务。
5. 开发者没有时间维护构建、生成代码、交叉编译和 CI。

判断公式: 如果以下三个问题至少两个回答“是”，才默认引入 Rust。

1. 端上是否存在复杂业务规则或高性能计算。
2. 本地数据是否需要成为长期可靠的事实源。
3. 这套内核是否有跨项目、跨平台复用价值。

否则先用 Flutter 单栈验证业务，再按瓶颈下沉 Rust。

---

## 4. 架构成熟度模型

顶级架构是渐进演进，不是一次性堆满。

| 阶段 | 目标 | Rust 参与度 | 典型能力 | 禁止事项 |
|------|------|-------------|----------|----------|
| M0 原型 | 验证产品价值 | 无或极少 | Flutter 单栈、Mock 数据 | 过早 DDD、CRDT、Actor |
| M1 稳定 MVP | 打通核心内核 | 低到中 | FRB、少量 Rust 算法、SQLite 初步 | FFI 细粒度调用 |
| M2 产品化 | 稳定可发布 | 中 | Rust repository、错误契约、埋点、分页 | 写后全量 reload |
| M3 增长期 | 数据和复杂度上升 | 中到高 | 离线队列、批量 DTO、Stream 事件、benchmark | 无界队列、无迁移测试 |
| M4 规模化 | 高复杂高可靠 | 高 | Snapshot、CRDT、Actor、OpenTelemetry | 无启用条件地堆技术 |
| M5 极端场景 | 专业/重型应用 | 很高 | ECS、端侧 AI、WASM SIMD | 用极端架构解决普通问题 |

SpendWhy / 碎片当前应按 M1 到 M2 推进，近期目标是 M2 产品化，而不是直接跳到 M4/M5。

### 4.1 核心决策树

#### 4.1.1 逻辑放在哪里

| 问题 | 是 | 否 |
|------|----|----|
| 是否影响业务事实源、权限、计费、同步、持久化一致性 | Rust Domain / Application | 继续判断 |
| 是否需要访问 SQLite、搜索索引、加密、文件、大批量数据 | Rust Infra | 继续判断 |
| 是否只是页面交互、输入草稿、动效、布局、未提交筛选条件 | Dart / Flutter | 继续判断 |
| 是否需要服务端实时裁决或频繁远程变更规则 | 服务端，端上只缓存和校验 | 继续判断 |
| 是否需要跨项目复用、离线运行或高性能计算 | Rust | Dart 可接受 |

默认规则: 影响事实源的逻辑不放 Widget；只影响像素和交互的逻辑不下沉 Rust。

#### 4.1.2 FFI 档位怎么选

| 条件 | 选择 |
|------|------|
| 纯函数、无 IO、无锁等待、无大分配、P99 可证明 < 100us | Sync |
| DB、网络、文件、加密、复杂业务、可能超过 1ms | Async |
| 进度、订阅、状态变化、后台任务事件 | Stream |
| 大数组、图片、音频、向量、导入导出二进制 | Zero-copy / chunk stream |

红线: 不因为“调用方便”选择 sync；不因为“实时”就无界 Stream；不因为“数据结构简单”就跨 FFI 传 JSON 主协议。

#### 4.1.3 存储怎么选

| 数据形态 | 默认选择 | 备注 |
|----------|----------|------|
| 结构化业务数据 | SQLite | 事务、索引、迁移成熟 |
| 高隐私结构化数据 | SQLCipher | 需配套密钥恢复策略 |
| 全文检索 | SQLite FTS5 | 先用内建能力 |
| 向量检索 | sqlite-vec 或专用库 | 先证明产品需要 |
| UI 偏好 | shared_preferences 或平台轻量存储 | 不参与业务查询和同步事实源 |
| 大文件 | Rust file service | 路径、权限、生命周期统一治理 |
| 临时缓存 | Dart 或 Rust 均可 | 命名必须带 cache，不能伪装事实源 |

#### 4.1.4 同步策略怎么选

| 场景 | 策略 |
|------|------|
| 单设备本地优先 | 本地事务 + 可选备份 |
| 多设备但冲突少 | 服务端裁决 + 本地队列 |
| 设置项 | LWW，但记录来源和时间 |
| 用户文本 | 双版本保留或规则合并 |
| 多人同时编辑同一对象 | 评估 CRDT |
| 金融、支付、账户安全 | 服务端权威，端上只做缓存和预校验 |

#### 4.1.5 高级能力启用闸门

| 能力 | 启用条件 | 退出条件 |
|------|----------|----------|
| Actor | 多个长期服务、独立状态机、锁竞争明显 | 单 repository 可清晰表达 |
| CRDT | 多端/多人离线编辑同一对象 | 服务端裁决足够、协作价值弱 |
| Snapshot + WAL | 全量重建超过启动预算 | DB 直读已达标 |
| OpenTelemetry | 本地日志和 Sentry 无法定位跨层问题 | 指标成本高于定位收益 |
| ECS 裸绘 | Widget 树无法承受海量节点 | 普通列表、表单、内容流 |

### 4.2 架构取舍矩阵

工程规则不是绝对真理，而是在特定阶段、约束和风险下的取舍。每次偏离默认方案时，必须说明放弃了什么。

| 默认选择 | 获得 | 代价 | 反向选择何时更好 |
|----------|------|------|------------------|
| Rust 做业务内核 | 正确性、复用、性能、隐私边界 | 工具链复杂、热重载变慢、CI 成本上升 | 极简原型、纯 CRUD、服务端强裁决 |
| Flutter 只做体验层 | UI 迭代快、边界清晰 | 需要设计 DTO 和事件协议 | UI-only 工具或一次性 demo |
| FFI 粗粒度 | 调用少、契约稳定、性能可控 | API 设计成本更高 | 极少量纯函数工具 |
| DTO 扁平化 | 序列化稳定、跨语言简单 | Dart 侧可能需要组装 view model | 小对象、低频管理页 |
| SQLite 作为事实源 | 查询强、事务成熟、迁移清楚 | schema 维护成本 | 临时缓存、纯文档对象存储 |
| Keyset pagination | 大数据稳定 | cursor 设计复杂 | 小数据、一次性后台工具 |
| Stream 增量事件 | 避免写后 reload、UI 更平滑 | 事件协议和取消复杂 | 小数据页面、低频设置页 |
| 乐观更新 | 体验跟手 | rollback 和 pending 状态复杂 | 支付、删除账户、密钥变更 |
| 本地优先 | 弱网可用、隐私更好 | 同步和冲突复杂 | 强服务端实时业务 |
| Sentry/Crashlytics 起步 | 成本低、见效快 | 跨层 trace 较弱 | 已有观测团队和平台 |
| 暂不上 CRDT | 保持简单 | 复杂协作体验有限 | 多人/多端同时编辑同一对象 |
| 暂不上 Actor | 调试简单 | 生命周期隔离能力有限 | 多长期服务和状态机并发协作 |

取舍记录规则:

1. 默认选择不需要 ADR，偏离默认选择需要 ADR。
2. 引入高级能力必须同时写退出条件。
3. 因短期效率违反边界，必须写入 `EXCEPTIONS.md`。
4. 如果反向选择连续出现三次，说明默认规则需要重新评估。

---

## 5. 总体分层

```text
Presentation      Flutter UI / Design System / Accessibility
Application       Dart Controller / UseCase Orchestration
State Boundary    Riverpod / Selector / View State
FFI Bridge        flutter_rust_bridge / DTO / Events
Rust Domain       Pure Logic / State Machine / Policy
Rust Infra        SQLite / Search / Crypto / Network / Files
Platform Adapter  iOS / Android capability adapters
Observability     Trace / Metrics / Crash / Logs across layers
```

### 5.1 层职责

| 层 | 应该做 | 不该做 |
|----|--------|--------|
| Presentation | 渲染、交互、动效、可访问性 | 业务规则、数据库、网络、重计算 |
| Application | 调用编排、节流、防抖、错误映射 | 持久化细节、算法细节 |
| State Boundary | 订阅、选择性 rebuild、缓存 view state | 业务事实源 |
| FFI Bridge | 类型边界、协议、事件流 | 手改生成代码、传 dynamic |
| Rust Domain | 状态机、规则、算法、纯函数 | 直接 IO |
| Rust Infra | DB、网络、搜索、加密、文件 | 产品交互规则 |
| Platform Adapter | 系统 API 适配 | 跨平台业务逻辑 |

### 5.2 核心边界

1. Flutter 不拥有业务事实源，只拥有 view state。
2. Dart Application 层只编排，不实现核心业务规则。
3. FFI API 表达用例，不表达字段访问。
4. Rust Domain 不直接依赖数据库、网络、文件系统。
5. Rust Infra 实现 IO，但不决定业务语义。
6. Platform Adapter 只处理系统能力，不承载跨平台业务。

### 5.3 允许留在 Dart 的内容

1. UI 临时输入。
2. 动效状态。
3. 页面筛选器的未提交状态。
4. 主题、语言、布局偏好。
5. 轻量 view cache。

### 5.4 必须优先下沉 Rust 的内容

1. 业务主数据。
2. 复杂状态机。
3. 权限与可见性规则。
4. 持久化一致性。
5. 加密与密钥相关逻辑。
6. 大规模查询、搜索、统计。
7. 离线同步与冲突处理。

### 5.5 Rust 内部分层规范

Rust 不是把所有代码都塞进一个 `api.rs`。推荐依赖方向如下:

```text
api -> application -> domain
api -> application -> infra
infra -> domain types
domain -> no api, no infra, no database, no network
```

推荐目录:

```text
rust/src/
  api/           # FRB 暴露入口，只做 DTO 转换和 tracing
  application/   # 用例编排、事务边界、权限校验、事件发布
  domain/        # 实体、值对象、状态机、策略、纯规则
  infra/         # SQLite、HTTP、文件、搜索、加密、平台适配
  observability/ # tracing、metrics、panic hook、日志桥接
```

依赖规则:

1. `domain` 不 import `rusqlite`、`sqlx`、`reqwest`、文件系统和 FRB 类型。
2. `api` 不写业务规则，只转换 DTO、创建 span、调用 use case。
3. `application` 决定事务边界和用例流程，但不拼 SQL。
4. `infra` 实现 repository trait，但不决定产品语义。
5. 跨模块公共类型优先放 `domain` 或专门的 `shared`，不要从 `api` 反向引用。

Repository 示例:

```rust
pub trait FragmentRepository {
    fn list(&self, filter: FragmentFilter, page: Page) -> Result<FragmentPage>;
    fn save(&self, fragment: &Fragment) -> Result<()>;
}

pub struct ArchiveFragmentUseCase<R: FragmentRepository> {
    repo: R,
}
```

事务规则:

1. 一个用户命令对应一个明确事务边界。
2. 跨 repository 写入必须由 application 层开启 Unit of Work 或显式事务。
3. 事务提交后再发布可见事件，避免 UI 看到未提交状态。
4. 读模型可以为 UI 优化，但必须能从事实源重建。

---

## 6. FFI 设计标准

### 6.1 四档调用模型

| 档位 | 用途 | 形式 | 起始预算 | 规则 |
|------|------|------|----------|------|
| Sync | 极轻纯函数 | `#[frb(sync)]` | P99 < 100us | 不做 IO、不等待锁、不大分配 |
| Async | 常规业务/IO | `pub async fn` | 交互路径 P99 < 16ms | 默认档 |
| Stream | 订阅/进度/事件 | `StreamSink<T>` | 持续增量 | 必须可取消、有背压 |
| Zero-copy | 大二进制/向量/图像 | `ZeroCopyBuffer` | 按吞吐衡量 | 必须 benchmark 验证复制路径 |

这些数字不是承诺，而是起始预算。真实预算必须在目标设备上测量。

### 6.2 用例式 API

错误示例:

```rust
pub async fn get_fragment(id: i64) -> FragmentDto;
pub async fn get_recoveries(fragment_id: i64) -> Vec<RecoveryDto>;
pub async fn get_tags(fragment_id: i64) -> Vec<TagDto>;
```

正确示例:

```rust
pub async fn load_fragment_detail(id: i64) -> Result<FragmentDetailView>;
pub async fn list_fragments(filter: FragmentFilter, page: Page) -> Result<FragmentBatch>;
pub async fn archive_fragment(command: ArchiveFragmentCommand) -> Result<CommandAck>;
pub fn watch_fragment_events(sink: StreamSink<FragmentEvent>) -> Result<()>;
```

API 名字应该像一个用户或系统用例，而不是字段访问器。

### 6.3 FFI 注释模板

```rust
/// FFI档位: Async
/// 用例: 加载碎片列表首页或下一页
/// 预算: profile 真机 P99 < 16ms，不含首次 DB cold start
/// 数据规模: limit <= 100，返回 FragmentBatch
/// 取消: 支持 request_id 取消
/// 观测: span = ffi.list_fragments
pub async fn list_fragments(filter: FragmentFilter, page: Page) -> Result<FragmentBatch> { ... }
```

### 6.4 反模式

1. Dart 循环中逐条调用 FFI。
2. 同一帧连续多次 sync FFI。
3. 跨 FFI 传 JSON 字符串作为主协议。
4. 返回深嵌套对象树。
5. Stream 没有 close/cancel。
6. FFI API 只服务某个临时 UI 细节。

### 6.5 flutter_rust_bridge 工程细节

#### 6.5.1 版本治理

1. `flutter_rust_bridge`、`flutter_rust_bridge_codegen`、生成的 Dart/Rust 代码必须成对升级。
2. FRB major/minor 升级必须单独提交，不与功能改动混合。
3. 升级后必须跑生成、Flutter analyze、Rust test、FFI integration test、Android 构建。
4. 文档中记录当前 FRB 版本、生成命令和已知限制。

#### 6.5.2 生成代码纪律

1. 生成代码不手改。
2. CI 检查生成代码是否过期。
3. API 删除字段必须先 deprecate，再删除。
4. mirror 类型和第三方类型映射必须集中管理。

#### 6.5.3 Isolate 与 Runtime

1. FRB 调用默认视为跨 runtime 边界调用，不假设它与 Flutter root isolate 同步执行。
2. Dart `compute()` 适合纯 Dart CPU 任务；Rust CPU 任务优先在 Rust 侧用 rayon 或 `spawn_blocking`。
3. Rust async 入口必须确认 Tokio runtime 初始化方式，避免重复创建 runtime 或在错误线程阻塞。
4. 所有长任务必须有 request_id / cancel token，页面退出时取消。

#### 6.5.4 Stream 取消范式

1. Dart 侧订阅必须在 dispose 中 cancel。
2. Rust 侧生产者必须能感知接收端关闭，并停止后台任务。
3. 高频事件使用 latest-wins、节流或批量合并，默认按 frame 级别而不是事件级别推送。
4. 进度事件只传必要字段: id、progress、phase、updated_ms、error_code。

#### 6.5.5 RustOpaque 生命周期

1. RustOpaque 必须有 owner、debug_id、释放入口和泄漏检测。
2. Dart `dispose` 是主释放路径，`Finalizer` / `NativeFinalizer` 只能作为兜底，不作为及时释放保证。
3. 不把 RustOpaque 存进全局 provider，除非它是进程级服务并有 shutdown 协议。
4. Release/profile 模式下单独验证路由反复进入退出后的 RSS 是否回落。

#### 6.5.6 平台构建与体积

1. Android 必须记录 NDK 版本、Rust targets、ABI splits、minSdk、16KB page size 兼容要求。
2. iOS 必须记录 Rust target、dSYM 上传、符号裁剪和 release 签名流程。
3. Rust release 配置可评估 `lto = true`、`codegen-units = 1`、`strip = true`、`panic = "abort"`。
4. 使用 `panic = "abort"` 会改变 panic 行为，不能再指望 unwind 被转成 Dart 异常。
5. 每次体积显著增长必须说明新增 crate、native lib、asset 或 debug symbol 的来源。

#### 6.5.7 跨边界类型规范

| 类型 | 跨边界表示 |
|------|------------|
| 时间点 | UTC i64 milliseconds，特殊场景可例外记录 |
| 日期 | `yyyy-mm-dd` string 或 days_since_epoch |
| 金额 | minor units i64 + currency code，不用 float |
| UUID / ULID | string |
| Decimal | string 或 scale + i128，按业务精度决定 |
| 地理坐标 | scaled integer 或 f64 pair，明确坐标系 |
| 大二进制 | Zero-copy 或 chunk stream |

### 6.6 FFI 反例库

错误: 为一个页面拼多个字段级调用。

```dart
final fragment = await api.getFragment(id);
final tags = await api.getTags(id);
final recoveries = await api.getRecoveries(id);
```

正确: Rust 提供页面用例视图。

```dart
final detail = await api.loadFragmentDetail(id: id);
```

错误: sync FFI 中读取文件。

```rust
#[frb(sync)]
pub fn load_config_text() -> String {
  std::fs::read_to_string("config.json").unwrap()
}
```

正确: IO 默认 async，并返回结构化错误。

```rust
pub async fn load_config() -> Result<AppConfig> { ... }
```

### 6.7 FRB 代码生成工作流（操作篇）

本节是 §6.5 的操作映射，回答“具体命令是什么、产物在哪里、谁依赖谁”。

#### 6.7.1 物理结构

flutter_rust_bridge 在本项目里的物理布局如下（与 cargokit 配套，见 §6.10）：

```text
SpendWhy/
  flutter_rust_bridge.yaml          # 生成器配置：rust_input / dart_output
  pubspec.yaml                      # 依赖 flutter_rust_bridge 运行时 + rust_lib_*
  lib/
    src/rust/                       # ← Dart 侧生成产物（不要手改）
      frb_generated.dart            #   入口 RustLib，加载 native lib
      frb_generated.io.dart         #   io 平台 FFI bindings
      frb_generated.web.dart        #   web 平台 bindings（如启用）
      api/                          #   每个 Rust api 模块对应一个 .dart
        dto.dart
        fade.dart
        view.dart
        recovery.dart
  rust/
    Cargo.toml                      # crate-type = ["cdylib","staticlib"]
    src/
      lib.rs                        # pub mod api; mod frb_generated;
      frb_generated.rs              # ← Rust 侧生成产物（不要手改）
      api/                          # ← 你手写的 FFI 入口（rust_input 根）
        mod.rs
        dto.rs
        fade.rs
        view.rs
        recovery.rs
        simple.rs                   # 含 #[frb(init)] 入口
      application/  domain/  infra/ # 业务层，不被 FRB 直接扫描
  rust_builder/                     # cargokit 提供的 Flutter 插件包
    pubspec.yaml                    # name: rust_lib_fragments
    android/  ios/  cargokit/
```

关键约束：

1. `rust_input` 指向的模块（本项目 `crate::api`）下的 `pub` 函数与类型才会被生成。
2. `frb_generated.rs` / `frb_generated.dart` / `frb_generated.io.dart` / `lib/src/rust/api/*.dart` 都是产物，**生成器输出原子提交**：四类文件必须同一个 PR 一起改。
3. `rust_builder/` 是供 Flutter 插件机制装载的“包装包”，不在它里面写业务代码，业务在 `rust/`。

#### 6.7.2 标准命令

```powershell
# 安装/升级生成器（与运行时版本必须严格一致，本项目 v2.12.0）
cargo install flutter_rust_bridge_codegen --version 2.12.0 --locked

# 生成（在仓库根执行，自动读取 flutter_rust_bridge.yaml）
flutter_rust_bridge_codegen generate

# 监视模式（开发期使用）
flutter_rust_bridge_codegen generate --watch

# 检查生成产物是否过期（CI 必跑）
flutter_rust_bridge_codegen generate --no-write
git diff --exit-code -- rust/src/frb_generated.rs lib/src/rust
```

CI 必须执行 `--no-write` + `git diff --exit-code` 检查；本地建议在 `lefthook` 的 `pre-push` 中加同样校验。

#### 6.7.3 产物责任表

| 文件 | 谁负责 | 何时变更 |
|------|--------|----------|
| `flutter_rust_bridge.yaml` | 人工 | 调整生成路径、引擎、codec 时 |
| `rust/src/api/**.rs` | 人工 | 新增/修改 FFI 用例时 |
| `rust/src/frb_generated.rs` | 生成器 | api 变更后 `generate` |
| `lib/src/rust/api/**.dart` | 生成器 | 同上 |
| `lib/src/rust/frb_generated*.dart` | 生成器 | 同上 |
| `lib/services/rust_backend.dart` | 人工 | 新增 Dart 侧门面、DTO 转换、schema 校验 |

> 规则：永远不要手改 `frb_generated.*`；如果手改能修好问题，说明生成配置或源 API 有缺陷，应改源头。

### 6.8 FRB 常用代码模式

以下模式覆盖本项目 95% 的实际需求。新增 API 时优先复用，避免发明私有约定。

#### 6.8.1 同步纯函数

```rust
#[flutter_rust_bridge::frb(sync)]
pub fn fade_level(
    fragment: FragmentDto,
    recoveries: Vec<RecoveryDto>,
    now_ms: i64,
) -> anyhow::Result<f64> { ... }
```

适用：纯计算、P99 < 100us。**禁止**在 `sync` 中做 IO、加锁、读文件。

#### 6.8.2 异步业务用例

```rust
pub async fn load_fragment_detail(
    ctx: TraceContext,
    id: String,
) -> anyhow::Result<FragmentDetailDto> { ... }
```

默认档位。任何 IO、DB、加密、网络都用 async，并接受 `TraceContext` 透传。

#### 6.8.3 Stream 增量事件

```rust
use flutter_rust_bridge::StreamSink;

pub async fn watch_fragment_events(
    sink: StreamSink<FragmentEvent>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    while let Some(evt) = next_event(&cancel).await {
        if sink.add(evt).is_err() { break; } // Dart 侧关闭
    }
    Ok(())
}
```

Dart 侧：

```dart
final sub = api.watchFragmentEvents().listen(_onEvent);
@override
void dispose() {
  sub.cancel(); // 必须取消，否则后台任务泄漏
  super.dispose();
}
```

#### 6.8.4 RustOpaque（长期持有的 Rust 对象）

```rust
#[flutter_rust_bridge::frb(opaque)]
pub struct SearchSession { /* 内部状态 */ }

impl SearchSession {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self { /* ... */ }

    pub async fn query(&self, q: String) -> anyhow::Result<SearchPage> { /* ... */ }

    #[flutter_rust_bridge::frb(sync)]
    pub fn dispose(&self) { /* 显式释放外部资源 */ }
}
```

Dart 侧 owner 必须在路由 `dispose` 时 `await session.dispose()`，**不依赖** GC。

#### 6.8.5 Mirror 第三方类型

不能给外部 crate 类型加 `#[frb]`，用 mirror 声明把它纳入生成范围：

```rust
use chrono::DateTime;

#[flutter_rust_bridge::frb(mirror(DateTime))]
pub struct _MirrorDateTime<Tz> { /* 字段镜像 */ }
```

镜像类型集中放在 `rust/src/api/mirror.rs`，避免散落。

#### 6.8.6 初始化入口

```rust
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils(); // 注册 panic hook、log 桥
    init_tracing();                                  // 见 §15.5
}
```

Dart 侧：

```dart
await RustLib.init(); // 加载 native lib + 调用 init_app
```

#### 6.8.7 错误传递

业务 Rust 函数返回 `anyhow::Result<T>` 或 `Result<T, AppError>`；FRB 自动把 `Err` 翻译成 Dart 异常 `AnyhowException` 或自定义异常。Dart 侧门面（如 `RustBackend`）负责把它再映射为 §11.3 的 UI 语义错误。

### 6.9 FFI 演进与版本协议

跨边界契约一旦发布给客户端就无法回收，必须有明确演进规则。

#### 6.9.1 字段级变更

| 操作 | 是否破坏性 | 操作步骤 |
|------|-----------|---------|
| 新增 optional 字段 | 否 | 直接加，不 bump schema_version |
| 新增 required 字段 | 是 | bump schema_version，旧客户端必须升级 |
| 删除字段 | 是 | 先 `#[deprecated]` 至少一个发布周期 → 下个周期再删 → bump schema_version |
| 重命名字段 | 是 | 等价于"新增 + 删除"，禁止直接改 |
| 改字段类型 | 是 | 新字段 + 双写 + 客户端切换 + 删旧字段 |
| 调整枚举取值 | 视情况 | 新增取值非破坏，删除/语义改变破坏 |
| 改 API 名 / 参数顺序 | 是 | 同重命名 |

#### 6.9.2 schema_version bump 规则

1. 每个跨 FFI DTO 必须有 `schema_version: u32`（已在 [rust/src/api/dto.rs](rust/src/api/dto.rs) 实施）。
2. Rust 侧导出 `supported_*_schema_version()` 同步函数；Dart 侧 `RustBackend.init` 启动期校验，不匹配立即 fail-fast。
3. 破坏性变更必须同时升 Rust 常量、Dart 期望常量、生成产物，三处同 PR。
4. 客户端在线时长跨度大的项目（如长期支持版），需要在 Rust 侧保留 N-1 schema 的 reader，做迁移再 bump。

#### 6.9.3 deprecation 流程

```rust
#[deprecated(since = "0.4.0", note = "use load_fragment_detail instead")]
pub async fn get_fragment(id: String) -> anyhow::Result<FragmentDto> { ... }
```

1. 标 `#[deprecated]`，CI 把 `deprecated` warning 视为 info 不阻断。
2. 在 `EXCEPTIONS.md` / changelog 写下下线日期。
3. 到期前确认所有 Dart 调用点已切换（`grep` + analyzer 检查）。
4. 删除 + bump schema_version + 升 codegen 内容 hash。

### 6.10 Android 平台构建细节

本项目使用 [`cargokit`](https://github.com/irondash/cargokit) 集成路径，由 `rust_builder` Flutter plugin 在 Gradle 构建期自动调 `cargo` 产出 `.so`，并放到 `jniLibs`。

#### 6.10.1 必须固定的版本

| 维度 | 来源 | 当前 / 推荐 |
|------|------|-------------|
| Flutter SDK | `flutter --version` / FVM | stable，记录到 README |
| AGP | `android/build.gradle.kts` | 与 Flutter 模板对齐 |
| Gradle | `android/gradle/wrapper/...` | 同上 |
| NDK | `android/local.properties` 或 `app/build.gradle.kts` | 与 cargokit 兼容版本 |
| Rust toolchain | `rust-toolchain.toml` | stable，固定 host |
| Rust Android targets | `rustup target list --installed` | `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` |
| minSdk / targetSdk | `app/build.gradle.kts` | 由 Flutter 模板提供，升级需复核权限 |

#### 6.10.2 已知陷阱

1. **Windows host = MSVC 时 Android 链接失败**：rustup 默认 host 解析到 `x86_64-pc-windows-msvc`，cargokit 调链时找不到 `link.exe`。必须 `rustup set default-host x86_64-pc-windows-gnu`，并把 Android targets 装在 stable toolchain 下。详见 [CASE-0002](#case-0002-windows-host-工具链影响-android-rust-构建)。
2. **16 KB page size**：Android 14+ 部分设备要求 `.so` 支持 16 KB 页对齐。新 NDK + 适配 linker flag 才能通过 Play 上架审查。
3. **ABI splits**：发布时按 ABI 分包（`splits.abi`）减小体积；CI 必须验证每个 ABI 都能加载。
4. **ProGuard / R8**：Rust 侧通过 FRB 生成的 JNI 入口符号不能被 R8 混淆掉；如启用 R8 需保留 `keep` 规则。

#### 6.10.3 构建产物校验

Release 构建后必须确认：

1. `build/app/outputs/flutter-apk/.../lib/<abi>/librust_lib_fragments.so` 存在。
2. `nm -D` 或 `llvm-readelf` 可看到 FRB 生成的导出符号。
3. App 启动日志包含 `[RustBackend] initialized`，**未出现** `init skipped ... librust_lib_fragments.so not found`（这是 FRB 加载失败的信号）。

### 6.11 iOS 平台构建细节

#### 6.11.1 集成方式

cargokit 在 iOS 构建期由 `rust_builder/ios/*.podspec` 触发 `cargo` 产出 `.a`（静态库），与 Flutter framework 一并链接进 `Runner.app`。

#### 6.11.2 必须固定的版本

| 维度 | 来源 | 推荐 |
|------|------|------|
| Xcode | CI / mac dev | 固定版本，记录到 README |
| iOS deployment target | `ios/Podfile` + Xcode | 与 Flutter 最低支持对齐 |
| Rust iOS targets | `rust-toolchain.toml` | `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios` |
| CocoaPods | `Gemfile` / 系统 | 固定版本 |

#### 6.11.3 已知陷阱

1. **Apple Silicon 模拟器**：必须有 `aarch64-apple-ios-sim` target，否则 M 系列 Mac 模拟器构建失败。
2. **Bitcode**：Xcode 14+ 默认禁用，Rust 静态库无需特殊处理；老项目升级时如仍开启 bitcode，需要 Rust 用 nightly 或专门 flag。
3. **dSYM 上传**：release 必须上传 dSYM 给 Sentry / Crashlytics，否则 Rust 侧崩溃栈无法符号化。
4. **App Store 隐私清单 (PrivacyInfo.xcprivacy)**：Rust 库若使用了 `Required Reason API`（如 `mach_absolute_time`），必须在隐私清单声明。

#### 6.11.4 启动校验

与 Android 一致：日志含 `[RustBackend] initialized`；`otool -L Runner.app/Runner` 可见静态链接的 Rust 符号。

### 6.12 Platform Channels 与 FRB 共存指引

FRB 不是替代 Flutter platform channels 的银弹。两者职责不同，混用是常态，混乱才是问题。

#### 6.12.1 各自适用范围

| 通道 | 适合 | 不适合 |
|------|------|--------|
| **FRB（Dart ↔ Rust）** | 业务内核、跨平台一致逻辑、高频/大数据、可复用算法、需要 Rust 生态（SQLite、加密、搜索、AI） | 调用平台 SDK、UI 系统服务、原生第三方 SDK |
| **Platform Channels（Dart ↔ Kotlin/Swift）** | 调用平台原生 API（相机、生物识别、推送、健康、应用内购买、深链）、原生第三方 SDK 包装、平台特定的 UI 组件（PlatformView） | 跨平台业务逻辑、复杂状态机、批量数据、长期内核 |
| **平台原生 → Rust（JNI / Swift FFI）** | 原生扩展内部需要复用 Rust 算法（如相机帧实时处理、PlatformView 内部使用 Rust） | 普通 Dart 调用路径 |

#### 6.12.2 决策树

调用一个原生能力时，按下面顺序判断：

```text
1. 这件事在 iOS 和 Android 上的语义是否一致？
     是 → 继续 2
     否 → Platform Channel + 各端原生实现，Dart 侧暴露统一 facade
2. 实现是否可以纯 Rust 完成（不依赖平台 API）？
     是 → FRB
     否 → 继续 3
3. 是否需要 Rust 处理结果（如解码图像、加密、入库）？
     否 → Platform Channel
     是 → Platform Channel 取数据 → Dart 转交 RustBackend → FRB 入 Rust
        （或：原生 → Rust JNI/Swift FFI 直传，避开 Dart）
```

#### 6.12.3 数据流模式

**模式 A**：Platform Channel 取，FRB 处理（推荐用于一次性请求）

```text
User Action
  → Dart Controller
  → Platform Channel: getRawHealthSamples()
  → Dart 拿到 List<HealthSample>
  → RustBackend.ingestHealthSamples()  ← FRB
  → Rust 入库 / 计算 / 触发事件
  → Dart Stream 更新 UI
```

**模式 B**：原生 → Rust 直传（推荐用于高频流，如相机帧）

```text
Camera plugin (Kotlin/Swift)
  → JNI 调 librust_lib_fragments.so 内部 C ABI 入口
  → Rust 处理后写共享 buffer
  → 通过 Platform Channel 或 FRB Stream 通知 Dart
```

**反模式**：Dart 在每帧之间用 `MethodChannel.invokeMethod` 把 byte[] 发给 Rust，再发回结果。会击穿 Dart isolate 消息队列。

#### 6.12.4 错误模型对齐

Platform Channel 和 FRB 的异常类型不同。Dart 侧门面（如 `RustBackend`、`PlatformBackend`）必须把它们都映射到 §11.3 的统一 UI 语义错误：

| 来源 | 原始异常 | 映射目标 |
|------|---------|---------|
| FRB | `AnyhowException` / 自定义 | `AppError`（按 code 分类） |
| Platform Channel | `PlatformException(code, message, details)` | `AppError`（code = `platform.<channel>.<code>`） |
| Plugin 不存在 | `MissingPluginException` | `AppError::Internal`（应在 doctor 里检测） |

不要把 `PlatformException.message` 直接展示给用户——它经常是英文的、面向开发者的。

#### 6.12.5 Trace 透传

Platform Channel 调用必须和 FFI 调用使用同一个 `trace_id`：

1. Dart Controller 在动作开始时生成 `trace_id`。
2. 调 platform channel 时把 `trace_id` 放进 args。
3. 原生侧用平台日志框架打一条 `[trace_id=...] channel.method start/end`。
4. 调 FRB 时把 `trace_id` 放进 `TraceContext`（见 §15.5）。
5. 上报的 crash / 错误日志都带同一个 `trace_id`，跨 4 层可串。

#### 6.12.6 不要做的事

1. 用 platform channel 实现跨平台业务逻辑（写两份 Kotlin/Swift 实现 = 立刻产生不一致）。
2. 用 FRB 包装平台 SDK（Rust 没有相机/通知/Push API；硬绕成 unsafe + JNI 是项目级灾难）。
3. 在 Rust 内 spawn 线程主动调 Dart（FRB 的 Stream 回调是反向 ABI 安全方式，自己 spawn JNI 线程 attach Dart isolate 不可控）。
4. 同一能力同时由 platform channel 和 FRB 提供两个入口（必有一个会过时）。

### 6.13 Flutter ↔ Rust 调用模式全景

本节是 §6.1（四档调用模型）与 §6.8（常用代码模式）的扩展和融合，作为**权威 cookbook**：每个常见场景给出"何时用 / Rust 模板 / Dart 模板 / 陷阱"。新增 FFI API 时优先复用本节模式；无法套用时再创新，并记入 ADR。

#### 6.13.0 模式选择速查表

| 你想做… | 推荐模式 | 章节 |
|--------|---------|------|
| 一次性纯计算（< 100us） | Sync 纯函数 | §6.13.1 |
| 一次性业务用例（DB / 加密 / 网络） | Async Future + 用例式 API | §6.13.2 |
| 持续接收事件 / 进度 | Stream + StreamSink | §6.13.3 |
| 用户可中途取消的长任务 | Async + CancellationToken | §6.13.4 |
| 任务进度报告 + 可取消 | Stream<Progress> + Cancel | §6.13.5 |
| 持有跨多次调用的状态对象 | RustOpaque + dispose | §6.13.6 |
| 大二进制 / 图像 / 向量 | Zero-copy buffer / chunk stream | §6.13.7 |
| 实时双向交互（如交互式 CLI） | 命令 Stream + 事件 Stream | §6.13.8 |
| Rust 主动通知 Dart（事件总线） | 全局 broadcast Stream | §6.13.9 |
| 同样请求短时合并 | Dart 侧 request coalescing | §6.13.10 |
| 列表分页 / 增量加载 | Keyset cursor + 批量 DTO | §6.13.11 |
| 重试 / 退避 / 幂等 | Dart 侧 wrapper + Rust idempotency key | §6.13.12 |
| Rust 调 Dart 注入的回调（IO 抽象、密钥提供） | Trait + Dart 实现 + RustOpaque | §6.13.13 |
| Dart 侧并发限流 / 资源池 | Mutex / Semaphore wrapper | §6.13.14 |
| 想在后台 isolate 跑 Rust | `compute` + RustLib.init | §6.13.15 |
| 启动期初始化 + 单例服务 | `frb(init)` + 全局 service | §6.13.16 |
| 按 feature mock Rust | `RustLib.initMock` + 接口分层 | §6.13.17 |

通用原则（贯穿所有模式）：

1. **API 表达用例，不表达 getter**（§6.2）。
2. **跨边界 DTO 必带 `schema_version`**（§6.9）。
3. **错误用 `Result<T, AppError>` 或 `anyhow::Result<T>`，不 panic**（§11）。
4. **所有 IO 默认 async；sync 只用于极轻纯函数**（§6.1）。
5. **所有长任务可取消、Stream 可关闭**（§10.2 / §10.3）。
6. **Dart 侧统一过 `RustBackend` 门面，UI / Provider 不直接 import 生成层**。
7. **每个调用都带 `trace_id`**（§15.5）。

---

#### 6.13.1 模式 A：同步纯函数（Sync）

**何时用**：纯计算，无 IO、无锁等待、无大分配，P99 < 100us，且调用方在 UI 线程能容忍同步阻塞。例如：评分公式、距离计算、字符串分类、配置查询。

**Rust 模板**

```rust
// rust/src/api/fade.rs
#[flutter_rust_bridge::frb(sync)]
pub fn fade_level(
    fragment: FragmentDto,
    recoveries: Vec<RecoveryDto>,
    now_ms: i64,
) -> anyhow::Result<f64> {
    let f = fragment.into_domain()?;
    let rs: Vec<_> = recoveries.into_iter()
        .map(RecoveryDto::into_domain)
        .collect::<AppResult<_>>()?;
    Ok(crate::domain::fade::fade_level(&f, &rs, now_ms))
}
```

**Dart 模板**（FRB 自动生成 `fade_api.fadeLevel`，业务再过门面）

```dart
// lib/services/rust_backend.dart
static double fadeLevel(Fragment f, List<Recovery> rs, {DateTime? now}) {
  final n = (now ?? DateTime.now()).toUtc().millisecondsSinceEpoch;
  return fade_api.fadeLevel(
    fragment: toFragmentDto(f),
    recoveries: rs.map(toRecoveryDto).toList(growable: false),
    nowMs: PlatformInt64Util.from(n),
  );
}
```

**陷阱**

1. `sync` **禁止** IO、锁、`tokio::block_on`、`std::fs`、网络。一旦阻塞会卡 UI 线程。
2. 别用 sync 跨 FFI 传 `Vec<Vec<T>>` 或大字符串——序列化开销可能超过算法本身。
3. sync 不能 `await` Rust async 代码；如果业务后期需要 IO，必须改 §6.13.2。
4. 一帧内不要连续触发 N 次 sync FFI。预计算成 batch 入口更便宜（§6.13.11）。

---

#### 6.13.2 模式 B：异步业务用例（Async Future）

**何时用**：默认档位。任何接触 DB、文件、加密、网络、可能 > 1ms 的业务用例。

**Rust 模板**

```rust
// rust/src/api/fragment.rs
pub async fn load_fragment_detail(
    ctx: TraceContext,
    id: String,
) -> anyhow::Result<FragmentDetailDto> {
    let span = tracing::info_span!(
        "ffi.load_fragment_detail",
        trace_id = %ctx.trace_id,
    );
    async move {
        let detail = crate::application::fragment::load_detail(&id).await?;
        Ok(FragmentDetailDto::from_domain(detail))
    }.instrument(span).await
}
```

**Dart 模板**

```dart
class RustBackend {
  static Future<FragmentDetail> loadFragmentDetail(String id) async {
    final dto = await fragment_api.loadFragmentDetail(
      ctx: _newTraceContext(screen: 'detail'),
      id: id,
    );
    return FragmentDetail.fromDto(dto);   // 门面再翻译成 Dart 模型
  }
}
```

**陷阱**

1. Rust async 函数默认在 FRB 内部 runtime 调度（基于 Tokio）。**不要**在 async 函数里 `block_on` 另一段 async，会死锁。
2. 长 IO 任务必须接 `CancellationToken`（§6.13.4），否则 Dart 取消订阅时 Rust 还在跑。
3. 不要让一个 async 函数返回**深嵌套**结构（如 `List<List<Map<String, Dto>>>`）——序列化代价高且类型脆弱，改成扁平列式 batch（§6.13.11）。

---

#### 6.13.3 模式 C：流式订阅（Stream）

**何时用**：持续推送，例如：业务事件总线、同步状态、下载进度、日志、实时指标。

**Rust 模板**

```rust
use flutter_rust_bridge::StreamSink;

pub async fn watch_fragment_events(
    sink: StreamSink<FragmentEventDto>,
) -> anyhow::Result<()> {
    let mut rx = crate::application::events::subscribe();
    while let Some(evt) = rx.recv().await {
        // sink.add 失败 = Dart 侧已取消订阅
        if sink.add(FragmentEventDto::from(evt)).is_err() {
            break;
        }
    }
    Ok(())
}
```

**Dart 模板**

```dart
class FragmentEventBus {
  StreamSubscription<FragmentEventDto>? _sub;

  void start(void Function(FragmentEvent) onEvent) {
    _sub = events_api.watchFragmentEvents().listen(
      (dto) => onEvent(FragmentEvent.fromDto(dto)),
      onError: (e, st) => debugPrint('[events] $e'),
    );
  }

  Future<void> dispose() async {
    await _sub?.cancel(); // 必须取消，否则 Rust 后台任务永远活着
    _sub = null;
  }
}
```

**陷阱**

1. **必须 cancel**。Provider/Page `dispose` 不取消订阅 = Rust 任务泄漏 + 内存上涨。
2. 高频事件（> 60Hz）必须**节流或合并**（latest-wins / batch by frame），否则击穿 Dart 微任务队列。
3. Stream 内传**事件**不传**完整快照**。完整快照请客户端拉一次，事件只描述增量。
4. Rust 侧 `sink.add(...).is_err()` 是检测下游关闭的唯一稳妥信号；不要依赖 `Drop` 的时机。

---

#### 6.13.4 模式 D：可取消的长任务

**何时用**：用户可能切页面或主动取消的任务，例如：导入文件、批量加密、训练嵌入。

**Rust 模板**

```rust
use tokio_util::sync::CancellationToken;

pub async fn import_archive(
    ctx: TraceContext,
    path: String,
    cancel: CancellationToken,
) -> anyhow::Result<ImportSummaryDto> {
    let mut count = 0u64;
    let mut stream = open_archive(&path).await?;
    while let Some(entry) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(AppError::cancelled("import_archive").into());
        }
        process_entry(entry?).await?;
        count += 1;
    }
    Ok(ImportSummaryDto { schema_version: 1, count })
}
```

> FRB 当前不直接生成 `CancellationToken`；常见做法是：把 token 包成 `RustOpaque<CancelHandle>`，Dart 侧持有句柄，调用 `handle.cancel()` 触发取消（§6.13.6 + §6.13.13）。

**Dart 模板**

```dart
final cancel = await rust.newCancelHandle();   // RustOpaque
try {
  final summary = await rust.importArchive(path: path, cancel: cancel);
  showOk(summary);
} on CancelledException {
  showInfo('已取消');
} finally {
  await cancel.dispose();   // 释放句柄
}

// 用户点击"取消"时
void onTapCancel() => unawaited(cancel.cancel());
```

**陷阱**

1. 取消是**协作式**的：Rust 必须主动检查 `cancel.is_cancelled()`，否则用户取消后任务还在跑。
2. 取消 ≠ 回滚。取消产生的部分副作用必须能被业务接受或显式回滚。
3. 别在每次 micro-step 都查 `is_cancelled()`，按业务节拍（如每条记录 / 每个 chunk）查一次。

---

#### 6.13.5 模式 E：进度 + 可取消（Stream + Cancel 组合）

**何时用**：用户既要看到进度，也要能取消。例如：大文件下载、批量同步、视频处理。

**Rust 模板**

```rust
#[derive(Clone)]
pub struct ProgressDto {
    pub schema_version: u32,
    pub total: u64,
    pub done: u64,
    pub phase: String,        // "downloading" | "verifying" | "indexing"
}

pub async fn run_sync(
    sink: StreamSink<ProgressDto>,
    cancel: CancellationToken,
) -> anyhow::Result<SyncResultDto> {
    let total = count_pending().await?;
    let mut done = 0u64;
    while let Some(op) = next_op().await? {
        if cancel.is_cancelled() { return Err(AppError::cancelled("sync").into()); }
        execute(op).await?;
        done += 1;
        // 节流：每秒最多 30 帧
        if should_emit() {
            let _ = sink.add(ProgressDto {
                schema_version: 1, total, done,
                phase: "syncing".into(),
            });
        }
    }
    Ok(SyncResultDto { schema_version: 1, done })
}
```

**Dart 模板**

```dart
final cancel = await rust.newCancelHandle();
final ctrl = StreamController<ProgressDto>.broadcast();

final task = rust.runSync(sink: ctrl.sink, cancel: cancel);
ctrl.stream.listen((p) => updateProgressBar(p.done / p.total));

try {
  final result = await task;
  showOk(result);
} on CancelledException {
  showInfo('已取消');
} finally {
  await ctrl.close();
  await cancel.dispose();
}
```

**陷阱**

1. 进度事件必须**节流**（按时间或按 done 增量），否则 1 秒能推 10000 条。
2. 进度 DTO 字段越少越好；不要在每条进度里塞业务对象。
3. Stream 错误不会自动取消任务；必须在 `onError` 里显式调 `cancel.cancel()`。

---

#### 6.13.6 模式 F：长生命周期句柄（RustOpaque）

**何时用**：跨多次调用复用的 Rust 对象，例如：搜索会话、加密上下文、模型推理 session、数据库连接池句柄。

**Rust 模板**

```rust
use std::sync::Arc;
use parking_lot::Mutex;

#[flutter_rust_bridge::frb(opaque)]
pub struct SearchSession {
    inner: Arc<Mutex<SearchEngine>>,
    debug_id: String,           // 便于泄漏排查
}

impl SearchSession {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new(index_path: String) -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(SearchEngine::open(&index_path)?)),
            debug_id: ulid::Ulid::new().to_string(),
        })
    }

    pub async fn query(&self, q: String, page: PageDto) -> anyhow::Result<SearchPageDto> {
        let engine = self.inner.clone();
        tokio::task::spawn_blocking(move || engine.lock().query(&q, page))
            .await?
            .map(SearchPageDto::from)
    }

    /// 显式释放，必须由 Dart owner 调用
    #[flutter_rust_bridge::frb(sync)]
    pub fn dispose(&self) {
        self.inner.lock().shutdown();
    }
}
```

**Dart 模板**

```dart
class SearchPage extends StatefulWidget { ... }
class _SearchPageState extends State<SearchPage> {
  late final SearchSession _session;

  @override
  void initState() {
    super.initState();
    _session = SearchSession(indexPath: '/path/to/index');
  }

  @override
  void dispose() {
    _session.dispose();   // 显式释放，不依赖 GC
    super.dispose();
  }
}
```

**陷阱**

1. **不要**把 RustOpaque 存进全局 provider，除非它是真正的进程级单例（§6.13.16）。否则页面退出无法及时释放。
2. **不要**依赖 Dart `Finalizer`/`NativeFinalizer` 作为主释放路径——GC 时机不可预测。`dispose` 是主路径，Finalizer 只兜底。
3. release/profile 模式必须**专门测试**：路由反复进出后 RSS 是否回落（§12.2）。
4. RustOpaque 内部访问业务状态必须自己加锁（`Mutex`/`RwLock`），FRB 不会自动同步。

---

#### 6.13.7 模式 G：大二进制（Zero-copy / Chunk Stream）

**何时用**：图像、音频、向量、模型输出、批量导出文件。任何超过 ~256KB 的数据。

**模式 G-1：Zero-copy 一次性返回**

```rust
use flutter_rust_bridge::ZeroCopyBuffer;

pub async fn render_thumbnail(id: String, w: u32, h: u32)
    -> anyhow::Result<ZeroCopyBuffer<Vec<u8>>>
{
    let bytes = render(id, w, h).await?;
    Ok(ZeroCopyBuffer(bytes))   // FRB 直接把内存所有权移交 Dart
}
```

```dart
final Uint8List bytes = await rust.renderThumbnail(id: id, w: 256, h: 256);
final image = await decodeImageFromList(bytes);
```

**模式 G-2：Chunk Stream（更大数据）**

```rust
pub async fn export_archive(
    sink: StreamSink<ZeroCopyBuffer<Vec<u8>>>,
) -> anyhow::Result<()> {
    let mut writer = ChunkWriter::new(64 * 1024);
    write_archive(&mut |chunk| {
        sink.add(ZeroCopyBuffer(chunk)).map_err(|_| AppError::cancelled("export"))
    }).await
}
```

```dart
final file = File('/sdcard/Download/export.zip').openWrite();
await for (final chunk in rust.exportArchive()) {
  file.add(chunk);
}
await file.close();
```

**陷阱**

1. Zero-copy 不是免费的——FRB 仍需一次内存所有权转移。**必须 benchmark** 证明它比普通返回更快。
2. 千万**不要**把同一份大数据在 Rust / Dart 同时持有副本（看 §12）。
3. Chunk 大小要适中：太小（< 4KB）系统调用次数多，太大（> 1MB）卡帧。常用 64KB ~ 256KB。
4. 不要把大二进制塞进普通 DTO（如 `Vec<u8>` 字段）——会被序列化成 base64 风格 copy。

---

#### 6.13.8 模式 H：实时双向交互（命令流 + 事件流）

**何时用**：用户与 Rust 服务持续对话，例如：交互式 REPL、AI 对话、实时编辑器协同。

**Rust 模板**

```rust
#[flutter_rust_bridge::frb(opaque)]
pub struct ChatSession {
    cmd_tx: tokio::sync::mpsc::Sender<ChatCommand>,
    evt_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<ChatEventDto>>>>,
}

impl ChatSession {
    #[flutter_rust_bridge::frb(sync)]
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(32);
        let (evt_tx, evt_rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(run_session(cmd_rx, evt_tx));
        Self { cmd_tx, evt_rx: Arc::new(Mutex::new(Some(evt_rx))) }
    }

    pub async fn send(&self, msg: String) -> anyhow::Result<()> {
        self.cmd_tx.send(ChatCommand::User(msg)).await?;
        Ok(())
    }

    /// 只能 listen 一次：取走内部 receiver。
    pub async fn events(&self, sink: StreamSink<ChatEventDto>) -> anyhow::Result<()> {
        let mut guard = self.evt_rx.lock();
        let mut rx = guard.take().ok_or_else(|| anyhow::anyhow!("already listening"))?;
        drop(guard);
        while let Some(evt) = rx.recv().await {
            if sink.add(evt).is_err() { break; }
        }
        Ok(())
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn dispose(&self) { /* 关闭 channel，等待后台任务退出 */ }
}
```

**Dart 模板**

```dart
final session = ChatSession();
final sub = session.events().listen(_onEvent);

await session.send(msg: 'hello');
// ...

@override
void dispose() {
  sub.cancel();
  session.dispose();
  super.dispose();
}
```

**陷阱**

1. mpsc 通道必须**有界**（如 capacity 32 / 128），背压触发时 Dart `send` 会等待。
2. 单 session 单 listener；多 listener 需要 `broadcast` 通道改造。
3. `dispose` 后 Dart 端不应再调 `send`；门面层做防御性检查。

---

#### 6.13.9 模式 I：Rust 主动通知 Dart（事件总线）

**何时用**：Rust 内任何模块发出业务事件，UI 全局订阅。例如：登录态变化、同步完成、通知到达。

**Rust 模板**

```rust
use once_cell::sync::OnceCell;
use tokio::sync::broadcast;

static EVENT_BUS: OnceCell<broadcast::Sender<AppEventDto>> = OnceCell::new();

pub(crate) fn publish(evt: AppEventDto) {
    if let Some(tx) = EVENT_BUS.get() {
        let _ = tx.send(evt);   // 没有订阅者也不报错
    }
}

#[flutter_rust_bridge::frb(init)]
pub fn init_event_bus() {
    let (tx, _) = broadcast::channel(256);
    let _ = EVENT_BUS.set(tx);
}

pub async fn subscribe_app_events(sink: StreamSink<AppEventDto>) -> anyhow::Result<()> {
    let mut rx = EVENT_BUS.get().unwrap().subscribe();
    loop {
        match rx.recv().await {
            Ok(evt) => if sink.add(evt).is_err() { break; },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                // 订阅者太慢：只发一个 warn，不断流
                let _ = sink.add(AppEventDto::lag_warning(n));
            }
            Err(_) => break,
        }
    }
    Ok(())
}
```

**Dart 模板**

```dart
final appEvents = rust.subscribeAppEvents().asBroadcastStream();
appEvents.listen(_global.handle);
// 多个 widget 都可以再 listen
```

**陷阱**

1. broadcast 有界 channel 必须处理 `Lagged`，不要直接 break。
2. 全局事件总线不要传**大对象**；只传 ID 或事件描述，详情按需再拉。
3. 不要让事件总线成为业务**主路径**——主路径用 §6.13.2 的明确 use case，事件总线只做"通知 UI 刷新"。

---

#### 6.13.10 模式 J：请求合并（Request Coalescing）

**何时用**：短时间内多次相同请求。例如：列表里 10 个 widget 都问"今天的天气"。

**Dart 模板**（Rust 不变，合并在 Dart 门面做）

```dart
class RustBackend {
  static final _inflight = <String, Future<WeatherDto>>{};

  static Future<WeatherDto> getWeather(String city) {
    return _inflight.putIfAbsent(city, () async {
      try {
        return await weather_api.getWeather(city: city);
      } finally {
        _inflight.remove(city);
      }
    });
  }
}
```

**陷阱**

1. 仅适合**幂等读**。写操作合并会丢请求。
2. key 设计要稳定（同一逻辑请求 key 必须相同）。
3. 与 §6.13.12 的重试组合时，要先合并再重试，否则一个失败拖垮所有调用方。

---

#### 6.13.11 模式 K：分页 / 增量加载（Keyset Cursor + 列式批量）

**何时用**：列表 / 时间线 / 搜索结果。任何潜在 > 100 条的数据。

**Rust 模板**

```rust
#[derive(Debug, Clone)]
pub struct PageDto {
    pub schema_version: u32,
    pub limit: u32,
    pub cursor: Option<String>,    // 上一页返回的 next_cursor
}

#[derive(Debug, Clone)]
pub struct FragmentBatch {
    pub schema_version: u32,
    pub ids: Vec<String>,
    pub created_ms: Vec<i64>,
    pub texts: Vec<String>,
    pub fade_levels: Vec<f64>,     // 列式：传输和反序列化都更快
    pub next_cursor: Option<String>,
}

pub async fn list_fragments(
    filter: FragmentFilterDto,
    page: PageDto,
) -> anyhow::Result<FragmentBatch> { ... }
```

**Dart 模板**

```dart
class FragmentListController {
  String? _cursor;
  bool _ended = false;

  Future<List<Fragment>> loadNext() async {
    if (_ended) return const [];
    final batch = await fragment_api.listFragments(
      filter: _filter, page: PageDto(schemaVersion: 1, limit: 50, cursor: _cursor),
    );
    _cursor = batch.nextCursor;
    _ended = batch.nextCursor == null;
    return List.generate(batch.ids.length, (i) => Fragment(
      id: batch.ids[i],
      createdAt: DateTime.fromMillisecondsSinceEpoch(batch.createdMs[i].toInt(), isUtc: true),
      content: batch.texts[i],
      fadeLevel: batch.fadeLevels[i],
    ));
  }
}
```

**陷阱**

1. **永远不要**用 `OFFSET` 跨大数据分页（§7.3）。
2. cursor 必须包含**唯一排序键**，例如 `(created_ms, id)`。
3. 列式 batch 比对象数组列表（`Vec<FragmentDto>`）在大数据上明显更快——序列化更紧凑、Dart 侧不会创建 N 个临时对象。
4. 不要每次重新 `await` 整个列表；只 `await` 增量。

---

#### 6.13.12 模式 L：重试 / 退避 / 幂等

**何时用**：网络写入、远程同步、可能因弱网失败的读。

**Rust 端**：所有写 API 必须接受**幂等键**（idempotency key），保证重试不重复。

```rust
pub async fn upload_fragment(
    op_id: String,                  // 客户端生成的幂等 key
    payload: FragmentDto,
) -> anyhow::Result<UploadAckDto> {
    if let Some(ack) = recall_recent(&op_id).await? {
        return Ok(ack);             // 已处理过，直接返回上次结果
    }
    let ack = do_upload(payload).await?;
    record_recent(&op_id, &ack).await?;
    Ok(ack)
}
```

**Dart 端**：在门面里包重试 wrapper：

```dart
Future<T> _withRetry<T>(Future<T> Function() fn, {int max = 3}) async {
  Object? lastErr;
  for (var i = 0; i < max; i++) {
    try { return await fn(); }
    on AppRetryableError catch (e) {
      lastErr = e;
      await Future.delayed(Duration(milliseconds: 200 * (1 << i)) + _jitter());
    }
  }
  throw lastErr!;
}

static Future<UploadAck> uploadFragment(Fragment f) {
  final opId = const Uuid().v4();
  return _withRetry(() => fragment_api.uploadFragment(opId: opId, payload: toDto(f)));
}
```

**陷阱**

1. 幂等键由**客户端**生成，并且要持久化（不要用临时 UUID，进程崩溃后要能续传）。
2. 退避必须有 **jitter**，避免雪崩。
3. 仅对 §11.3 中标记为 `Retryable` 的错误重试；`Validation`/`Conflict`/`Auth` 不重试。

---

#### 6.13.13 模式 M：Rust 调 Dart 注入的回调（依赖反转）

**何时用**：Rust 业务核心需要某个能力，但实现在 Dart 一侧。例如：从平台 Keystore 读密钥、弹出系统对话框、上报埋点。

**Rust 模板**：用 trait + Dart 提供实现，调用通过命令 channel + 事件 channel 异步串。

```rust
// Rust 侧：定义抽象
#[async_trait::async_trait]
pub trait SecretProvider: Send + Sync {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
}

// 让 Dart 提供实现：用 mpsc 命令 + oneshot 回应
pub struct DartSecretProvider {
    tx: mpsc::Sender<(String, oneshot::Sender<Option<String>>)>,
}

#[async_trait::async_trait]
impl SecretProvider for DartSecretProvider {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send((key.to_string(), resp_tx)).await?;
        Ok(resp_rx.await?)
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct SecretBridge { /* 持有 mpsc::Receiver 端 */ }

impl SecretBridge {
    pub async fn next_request(&self, sink: StreamSink<SecretRequestDto>) -> anyhow::Result<()> { ... }

    pub async fn respond(&self, request_id: String, value: Option<String>) -> anyhow::Result<()> { ... }
}
```

**Dart 模板**：把 Dart 实现挂到 Rust 抽象上。

```dart
final bridge = SecretBridge();
bridge.nextRequest().listen((req) async {
  final v = await FlutterSecureStorage().read(key: req.key);
  await bridge.respond(requestId: req.requestId, value: v);
});

// Rust 内部业务路径就可以用 SecretProvider 了
```

**陷阱**

1. 这是**最后选择**。能在 Rust 内完成的就别走回 Dart。
2. 必须设超时；Rust 不要无限期等 Dart 回应。
3. 注意循环死锁：Dart 调 Rust，Rust 又同步等 Dart，必须用上面的 mpsc 异步模式而不是阻塞等。

---

#### 6.13.14 模式 N：并发限流 / 资源池

**何时用**：Rust 内有有限资源（如数据库连接、GPU、HTTP 并发上限）。

**Rust 模板**

```rust
use tokio::sync::Semaphore;
static DB_LIMIT: OnceCell<Arc<Semaphore>> = OnceCell::new();

pub async fn run_query(sql: String) -> anyhow::Result<QueryResultDto> {
    let sem = DB_LIMIT.get_or_init(|| Arc::new(Semaphore::new(4)));
    let _permit = sem.acquire().await?;     // 最多 4 并发
    do_query(sql).await
}
```

**陷阱**

1. 限流应在**离资源最近**的层做（infra 层），不在 api 层。
2. 不要用 `Mutex` 模拟限流——`Mutex` 是串行化，Semaphore 才是并发上限。
3. Dart 侧也要懂得排队：同一资源短时大量并发请求，可在门面里加 `Throttle`/`Queue`。

---

#### 6.13.15 模式 O：在后台 Isolate 调 Rust

**何时用**：超长 CPU 任务（解码、AES 大文件、嵌入计算），不希望污染 root isolate 即使是 async。

**Dart 模板**

```dart
Future<EmbedResult> embedInBackground(String text) async {
  return Isolate.run(() async {
    await RustLib.init();           // 子 isolate 必须自己 init
    return embed_api.embed(text: text);
  });
}
```

**陷阱**

1. 子 isolate **必须**自己调 `RustLib.init()`；它和 root isolate 不共享 FRB 状态。
2. 子 isolate 内调用 RustOpaque 创建的对象**不能**传回 root isolate（句柄绑定 isolate）。
3. 简单 CPU 任务 Rust 内 `spawn_blocking` 就够了，不必上 Dart isolate。
4. 数据要 **`compute`-friendly**：参数和返回值必须可以在 isolate 间传输。

---

#### 6.13.16 模式 P：进程级单例服务

**何时用**：登录态、配置、连接池、事件总线、追踪 subscriber 等"应用级"对象。

**Rust 模板**

```rust
use once_cell::sync::OnceCell;

pub struct AppCore { /* repos, config, event bus */ }
static CORE: OnceCell<AppCore> = OnceCell::new();

#[flutter_rust_bridge::frb(init)]
pub fn init_app() -> anyhow::Result<()> {
    flutter_rust_bridge::setup_default_user_utils();
    init_tracing();                     // §15.5
    let core = AppCore::bootstrap()?;   // 阻塞启动，注意预算（§13.1）
    CORE.set(core).map_err(|_| anyhow::anyhow!("already initialized"))?;
    Ok(())
}

pub(crate) fn core() -> &'static AppCore {
    CORE.get().expect("app core not initialized")
}
```

**Dart 模板**：[`lib/services/rust_backend.dart`](lib/services/rust_backend.dart) 已经是这个模式：`RustLib.init()` 一次，全程通过门面访问。

**陷阱**

1. `frb(init)` 阻塞启动，必须**严格控制耗时**（§13.1）。重 IO 放后台预热而不是 init。
2. 单例**不能**用 `RustOpaque` 暴露给 Dart——句柄绑定 owner，会导致进程内多个 owner 互相覆盖。让 Dart 通过普通 API 调用即可。
3. 单例对象的并发：多 async 调用同时进单例需要内部锁。

---

#### 6.13.17 模式 Q：测试与 mock

详见 §16.4。要点回顾：

1. **Dart unit test**：用 `RustLib.initMock(api: mockApi)`，mock 生成的 `RustLibApi`，**不**走 native lib。
2. **Rust unit test**：在 `domain` / `application` 内 `#[cfg(test)]`，根本不经 FFI。
3. **Integration test**：放 `integration_test/`，真加载 native lib，覆盖 schema 校验、错误传递、RustOpaque dispose 等只有真调才能发现的问题。

---

#### 6.13.18 通用陷阱清单

不论选哪个模式，以下问题都必须答得出：

1. 这个 API 取消时会发生什么？谁负责释放资源？
2. 这个 API 的最大输入规模是多少？有没有上限？
3. 这个 API 的 P99 预算是多少？怎么测？
4. 失败模式有哪些？分别映射到哪种 `AppError`？
5. 重试是否安全？是否需要幂等键？
6. 跨 isolate 是否还能用？
7. RustOpaque 的 owner 是谁？dispose 谁负责？
8. Trace context 怎么传？crash 时怎么定位回这次调用？

把这八问写进 §6.3 的 FFI 注释模板里，作为 PR 自检项。

---

## 7. 数据建模与持久化

### 7.1 DTO 戒律

1. DTO 是跨边界契约，不是 Rust 内部模型。
2. DTO 必须带 `schema_version`。
3. 跨 FFI 对象最多一层嵌套。
4. 时间统一 i64 毫秒。
5. 枚举跨边界要稳定，优先 code + Dart 映射。
6. 高频聚合在 Rust 预计算。
7. 大列表优先列式批量结构。
8. 业务内部模型允许丰富，跨边界模型必须克制。

列式批量结构示例:

```rust
pub struct FragmentBatch {
    pub schema_version: u32,
    pub ids: Vec<i64>,
    pub created_ms: Vec<i64>,
    pub texts: Vec<String>,
    pub fade_levels: ZeroCopyBuffer<Vec<f64>>,
    pub flags: Vec<u32>,
    pub next_cursor: Option<i64>,
}
```

### 7.2 默认存储策略

| 类型 | 推荐 |
|------|------|
| 结构化业务数据 | SQLite / SQLCipher |
| Rust 绑定 | rusqlite 或 sqlx，按复杂度选择 |
| 全文检索 | FTS5 |
| 向量检索 | sqlite-vec 或专用本地向量库 |
| KV 配置 | redb/sled 或平台轻量存储 |
| 大文件 | Rust file service |
| UI 偏好 | Dart shared preferences 可接受 |

### 7.3 查询规则

1. 大列表默认 keyset pagination。
2. OFFSET 只用于小数据、后台管理或一次性工具。
3. 高频查询必须有索引。
4. 写入必须事务化。
5. 批量写入必须走批量 API。
6. 查询计划需要在数据规模上升前审查。
7. Schema 变更必须有迁移测试。

### 7.4 数据完整性

1. SQLite 开启 WAL 前必须理解备份策略。
2. 关键写入后需要明确 transaction 与 fsync 语义。
3. App 启动时应能检测 schema 版本、迁移状态和损坏风险。
4. 数据库损坏时提供降级路径: 备份恢复、只读模式、导出残留数据。
5. 破坏性迁移必须有备份或可恢复策略。

迁移目录建议:

```text
rust/migrations/
  0001_init.sql
  0002_add_fragment_index.sql
  0003_add_deleted_at.sql
```

### 7.5 ID、时间与精度

1. 离线优先或多设备同步对象默认使用客户端可生成的全局 ID，例如 UUIDv7 / ULID。
2. 单设备本地表可以使用 SQLite integer primary key，但跨设备同步前必须有稳定 global_id。
3. 事件时间统一 UTC，展示时才转本地时区。
4. 业务日期和时间点分开建模，生日、账期、日历日不要用 timestamp 偷懒。
5. 金额、分数、比例等精度敏感数据禁止用 float 表示事实源。
6. 排序游标要稳定，推荐 `(created_ms, id)` 或专门 cursor，避免同毫秒数据丢失。

### 7.6 数据库连接与并发

1. SQLite 写入必须串行化或通过连接池策略明确约束。
2. `rusqlite` 同步连接适合简单本地库；长查询需放到阻塞线程，避免卡 async runtime。
3. `sqlx` 适合复杂异步场景，但要控制连接池大小，移动端不是服务端。
4. 单连接、连接池、读写分离必须写入 ADR，不能靠默认值。
5. 每个高频查询要能解释索引命中情况。

---

## 8. 状态一致性与用户体验

### 8.1 单一事实源

Rust repository 是业务主数据事实源。Flutter state 是投影，不是事实源。

标准流:

```text
User Action -> Dart Controller -> Rust Command -> Mutation -> Event -> Riverpod State -> UI
```

写操作不手动全量 reload。任何 `await write(); await load();` 都是需要审查的信号。

### 8.2 Riverpod 规则

1. 页面只 watch 自己需要的字段。
2. 大对象用 selector，不整对象订阅。
3. 列表状态使用不可变集合或明确 copy-on-write 策略。
4. Controller 不做业务规则，只做调用编排。
5. 错误、加载、空状态必须是一等状态，不靠 null 猜。

### 8.3 乐观更新

乐观更新用于提升体验，不是替代一致性。不要承诺“0 延迟”，只承诺“先给可纠正的本地反馈”。

启用条件:

1. 用户动作成功概率高。
2. 失败可解释、可回滚。
3. UI 有明确 pending 状态。
4. Rust 返回 action_id 对应结果。

标准协议:

```rust
pub struct ClientAction {
    pub action_id: i64,
    pub created_ms: i64,
    pub payload: ActionPayload,
}

pub enum ActionResult {
    Accepted { action_id: i64 },
    Rejected { action_id: i64, error: AppError },
    RolledBack { action_id: i64, reason: String },
}
```

禁止事项:

1. 不可回滚的破坏性操作不做乐观更新。
2. 涉及支付、删除账户、密钥变更，不做无确认乐观更新。
3. 乐观 UI 必须能被最终 Rust 事件纠正。

### 8.4 Stream 与 Signal 的取舍

不要绝对化“全局 Stream 错、Signal 对”。按规模选择:

| 规模 | 推荐方案 |
|------|----------|
| 小型 App | 聚合 Stream + selector |
| 中型 App | feature/topic 粒度事件流 |
| 大型 App | entity-id 粒度事件 + patch |
| 极高频状态 | latest-wins signal + frame throttle |

所有高频事件都必须节流或合并，避免 Dart 微任务队列和 GC 被击穿。

---

## 9. 离线优先与同步

### 9.1 定义

用户在无网、弱网、网络切换时仍能完成核心任务。网络用于同步和增强，不是核心体验的前置条件。

### 9.2 本地写入流程

1. 校验输入。
2. 写入本地事务。
3. 记录 pending sync op。
4. 推送本地事件更新 UI。
5. 按 9.5 的同步触发器执行后台同步。
6. 成功标记 synced，失败进入 retry/conflict。

同步状态模型:

```rust
pub enum SyncState {
    LocalOnly,
    PendingUpload,
    Synced,
    Conflict,
    FailedRetryable,
    FailedFatal,
}
```

### 9.3 冲突策略

| 场景 | 默认策略 |
|------|----------|
| 用户文本 | 保留双版本，交给用户或规则合并 |
| 计数/统计 | CRDT 或服务端重算 |
| 设置项 | last-write-wins，但记录时间和来源 |
| 删除 | tombstone，不立即物理删除 |

### 9.4 CRDT 启用条件

CRDT 是强工具，但不是默认工具。

适用:

1. 多设备或多人同时编辑同一对象。
2. 离线修改必须无中心协调合并。
3. 冲突自动合并价值高于复杂度。

不适用:

1. 单设备本地优先 App。
2. 冲突可以由服务端简单裁决。
3. 数据量大但协作需求弱。
4. 团队无法维护 op log GC、schema 演进和调试工具。

治理要求:

1. Tombstone / op log GC。
2. 快照压缩。
3. 冲突可视化。
4. 迁移测试。
5. 内存上限监控。

### 9.5 服务端与网络契约

即使端上是核心，服务端仍然常见。默认边界如下:

1. Rust Infra 负责 HTTP/gRPC 客户端、重试、超时、token 刷新、断点续传、响应解析。
2. Dart 不直接调用业务服务端 API，除非是平台 SDK 或非核心分析上报。
3. 服务端返回必须映射为稳定 `AppError`，不把 HTTP status 原样泄漏到 UI。
4. 所有写请求必须有幂等键或本地 op_id，避免弱网重试造成重复写。
5. 网络请求必须有超时、取消、重试上限、指数退避和 jitter。
6. 认证刷新必须单飞，禁止并发刷新 token 互相覆盖。
7. 大文件上传下载必须支持断点、校验和、取消和后台恢复策略。

网络错误语义:

| 错误 | UI 策略 | 后台策略 |
|------|---------|----------|
| Timeout | 可重试提示或静默重试 | 指数退避 |
| 401 | 触发登录恢复 | 暂停队列 |
| 403 | 明确权限提示 | 不重试 |
| 409 | 进入冲突处理 | 保留本地版本 |
| 429 | 降频提示 | 按服务端 retry-after |
| 5xx | 轻提示或状态条 | 退避重试 |

同步触发器:

1. App 前台恢复。
2. 网络从不可用变为可用。
3. 本地写入后短延迟。
4. 用户手动同步。
5. 后台任务窗口，平台允许时执行。

---

## 10. 并发、取消与背压

### 10.1 默认选择

| 场景 | 工具 |
|------|------|
| IO 密集 | Tokio async |
| CPU 密集 | rayon / spawn_blocking |
| 单一状态订阅 | watch channel |
| 多事件广播 | broadcast/mpsc 有界通道 |
| 多读少写 | arc-swap / RwLock，视竞争情况 |
| 复杂生命周期隔离 | Actor |

### 10.2 取消协议

长任务必须可取消。取消不是优化，是正确性。

```rust
tokio::select! {
    _ = cancel.cancelled() => Err(AppError::cancelled()),
    result = do_work() => result,
}
```

### 10.3 背压策略

上游快于下游时，必须选择一个策略:

1. 合并: 多个进度事件合并成最新值。
2. 丢弃旧值: UI 只关心 latest state。
3. 阻塞: 数据不可丢，如文件写入。
4. 拒绝: 返回 overload 错误。

不允许无界队列作为默认方案。

### 10.4 Actor 启用条件

Actor 用于隔离生命周期和状态域，不用于掩盖混乱设计。

适用:

1. 多个长期运行服务并发协作。
2. 每个服务有独立状态机。
3. 需要监督、重启、优先级队列。
4. 锁竞争和生命周期管理已成为主要复杂度。

不适用:

1. 简单 CRUD。
2. 单 repository 可以清晰解决的问题。
3. 开发者还没有稳定事件模型。

---

## 11. 错误、崩溃与恢复

### 11.1 基本原则

生产路径使用 `Result<T, AppError>`。panic 代表 bug，不是业务控制流。

### 11.2 错误契约

```rust
pub enum AppError {
    Validation { code: String, message: String },
    NotFound { code: String },
    Conflict { code: String, retryable: bool },
    Io { code: String, retryable: bool },
    Cancelled { code: String },
    Internal { code: String },
}
```

Dart 侧必须把错误映射成用户语义:

1. 可重试。
2. 需用户操作。
3. 可忽略。
4. 致命错误。

### 11.3 错误到 UI 的映射

| 错误语义 | UI 表达 | 是否重试 | 例子 |
|----------|---------|----------|------|
| Validation | 表单 inline error | 用户修改后重试 | 文本为空、格式错误 |
| NotFound | 空态或返回上一层提示 | 通常不重试 | 数据已删除 |
| Conflict | 冲突解决视图 | 用户选择后重试 | 多端编辑冲突 |
| Retryable IO | 状态条 / 轻提示 | 自动退避 + 手动重试 | 弱网、超时 |
| Auth | 登录恢复流程 | 重新认证后重试 | token 过期 |
| Fatal | 错误页 + 导出/反馈入口 | 不自动重试 | 数据损坏、版本不兼容 |

UI 不展示底层错误栈，不展示未脱敏路径、SQL、token、用户隐私字段。

### 11.4 错误规则

1. 生产业务路径禁止裸 `unwrap()` / `expect()`。
2. 静态证明安全的 unwrap 允许，但必须靠近原因说明。
3. 禁止吞错，`let _ =` 必须说明为什么可忽略。
4. 所有跨 FFI 错误必须有稳定 code。
5. 用户可见错误必须区分: 可重试、需操作、不可恢复。

### 11.5 Panic 与 native crash

不要把 FFI 边界的 panic 捕获当作可靠容错。不同 FRB 版本、panic 策略和平台行为可能不同，以下问题可能绕过普通异常路径:

1. abort panic 策略。
2. 栈溢出。
3. C/C++ 库段错误。
4. FFI 指针生命周期错误。
5. OOM。

因此必须:

1. 避免业务路径 panic。
2. 限制递归深度。
3. 管理 RustOpaque 生命周期。
4. 接入 crash reporting。
5. 为关键数据写入提供恢复策略。

### 11.6 Unsafe Rust 边界

`unsafe` 是 Rust 语言提供的安全闸门，不是“临时逃生口”。一旦进入 unsafe，内存安全、别名规则、生命周期、线程安全都变成人的责任。

#### 11.6.1 何时可以写 unsafe

默认全项目业务 crate **禁用** unsafe。仅以下十分明确的场景才考虑：

1. 包装 C/C++ 库的 FFI bindings（不包括 FRB——FRB 已为你隐藏 unsafe）。
2. 高性能 SIMD / 手写内存布局优化，并且被 benchmark 证明收益。
3. 与平台原生 API 交互，必须传递原始指针（如 Android `AHardwareBuffer`）。
4. 实现 lock-free 数据结构（全项目必须走 ADR）。

不可以用 unsafe 的场景：“绕过 borrow checker”、“临时处理生命周期问题”、“为了快一点”。

#### 11.6.2 代码纪律

```rust
// SAFETY: 调用者保证 ptr 指向合法初始化的长度为 `len`
//   的 u8 数组，在本函数返回前不被别的线程释放。
//   本函数只读不写，不产生可变别名。
#[allow(unsafe_code)]
unsafe fn parse_borrowed(ptr: *const u8, len: usize) -> Result<View<'_>> {
    debug_assert!(!ptr.is_null());
    debug_assert!(len <= isize::MAX as usize);
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    parse(slice)
}
```

硬规则：

1. 每个 `unsafe` 块紧邻一个 `// SAFETY:` 注释，列出调用者必须满足的不变量。没有 SAFETY 注释 = clippy 报错（`clippy::undocumented_unsafe_blocks`）。
2. `unsafe fn` 的 doc comment 的第一段必须是 `# Safety` 节，说明调用者义务。
3. unsafe 块里只放真正需要的一行，不要把安全代码也包进去，避免净区占领审查注意力。
4. 全项目顶层推荐 `#![deny(unsafe_op_in_unsafe_fn)]`，强制 unsafe fn 内部仍需显式标 unsafe block。
5. 任何 unsafe 提交必须在 PR 描述里明确陈述：为什么不能安全实现、哪些不变量被破坏会 UB、如何被测试。

#### 11.6.3 不变量与文档

每个包含 unsafe 的模块须在顶部文档注释中维护一份 **不变量清单**：

```rust
//! # 模块不变量
//!
//! 1. `Buf::ptr` 始终指向长度为 `Buf::cap` 的初始化内存。
//! 2. `Buf::len <= Buf::cap` 始终成立。
//! 3. 该类型不实现 `Send`；Dart 侧只能在创建的线程访问。
//!
//! 调用与修改 `pub(crate)` API 时必须逐条检查以上不变量是否仍然成立。
```

#### 11.6.4 验证手段

| 手段 | 何时跑 | 说明 |
|------|--------|------|
| `cargo clippy -- -D warnings` | 本地 / CI 必跑 | 开启 `clippy::undocumented_unsafe_blocks`、`clippy::missing_safety_doc` |
| `cargo +nightly miri test` | unsafe 代码修改后 | 检测 UB、越访、未初始化读、别名违反 |
| `RUSTFLAGS="-Z sanitizer=address"` | unsafe 代码修改后 | 检测堆越界 / use-after-free，仅 nightly + 部分 target |
| `cargo +nightly fuzz` | 解析器 / 不受信输入 | 检测 panic、UB、越界 |
| `cargo deny` | 依赖升级 | 拦截引入带 unsafe 漏洞的依赖 |

#### 11.6.5 ADR 规则

任何新增 unsafe 代码必须伴随 ADR，至少回答：

1. 为什么 safe Rust 不够，benchmark / API 证据是什么。
2. 哪些不变量被依赖，不变量被破坏后最坏后果是什么（UB / panic / 崩溃 / 数据丢失）。
3. miri / sanitizer / fuzz 覆盖到什么程度。
4. 退出路径：如果将来 safe 替代出现（如标准库稳定某个 API），是否愿意迁回。

---

## 12. 内存与大对象

### 12.1 大对象传输

大对象包括图片、音频、向量、模型输出、导入导出文件、批量图表数据。

规则:

1. 优先二进制协议，不走 JSON。
2. 优先 zero-copy 路径，但必须 benchmark 验证。
3. 明确对象生命周期。
4. 避免 Dart 和 Rust 同时持有多份大对象。
5. 大文件走 stream/chunk，不一次性读入内存。

### 12.2 RustOpaque

RustOpaque 适合长期持有 Rust 对象句柄，但必须有释放策略。

要求:

1. Dart owner 明确。
2. 页面 dispose 时释放。
3. Rust 侧可检测泄漏。
4. 长期对象有 debug id。
5. 测试路由反复进入退出时内存是否回落。

---

## 13. 冷启动与性能工程

### 13.1 启动分层

启动任务分三类:

1. 必须阻塞首帧: Flutter binding、主题、必要配置、Rust 基础初始化。
2. 必须阻塞首屏数据: 首屏 repository、用户本地状态。
3. 可延后: 索引预热、同步、分析、缓存清理、模型加载。

至少记录:

1. app_start。
2. rust_init_start/end。
3. first_frame。
4. first_screen_data_ready。
5. first_interaction_ready。

### 13.2 性能预算

| 指标 | 起始目标 |
|------|----------|
| 冷启动到首帧 | < 1.5s |
| 首屏可交互 | < 2.5s |
| sync FFI P99 | < 100us |
| async FFI 交互路径 P99 | < 16ms |
| 大列表滚动 | 稳定 60fps，尽量适配高刷新率 |
| 1 万条数据常驻内存 | < 80MB，按业务校准 |
| arm64 release 体积 | < 25MB，超出需说明 |
| crash-free 用户率 | > 99.5% |

说明: “1 万条数据常驻内存 < 80MB”默认指常规列表 read model，不含图片缓存、全文索引、向量索引、模型文件和大二进制缓存。特殊数据形态必须单独建立预算。

### 13.3 测量方法

1. 同一设备。
2. 同一构建模式。
3. 同一数据规模。
4. 至少 30 次冷启动样本或 1000 次微基准样本。
5. 丢弃预热样本。
6. 报告 P50/P95/P99。
7. 记录设备温度、电量、是否充电、是否后台干扰。
8. 与基线 commit 对比。

任何性能优化 PR 必须说明优化前数据、优化后数据、测量方式、受影响设备、代价和风险。

### 13.4 SLI、SLO 与错误预算

性能预算是工程目标，SLO 是发布契约。SLO 失败不是“以后优化”，而是触发明确动作。

| 能力 | SLI | SLO | 触发动作 |
|------|-----|-----|----------|
| 稳定性 | crash-free users | > 99.5% | 停止新功能发布，优先修复崩溃 |
| 冷启动 | first_frame P95 | < 1.5s | 分析启动瀑布图，延迟非必要初始化 |
| 首屏可用 | first_screen_ready P95 | < 2.5s | 优化首屏 read model，拆分阻塞任务 |
| FFI 交互 | async FFI P99 | < 16ms | 标记慢 API，拆分慢路径或后台化 |
| Sync FFI | sync FFI P99 | < 100us | 禁止 IO/锁等待/大分配，必要时改 async |
| 滚动体验 | jank frames ratio | < 1% | 检查 rebuild、列表虚拟化、图片缓存 |
| 同步恢复 | pending op age P95 | < 5min | 检查队列、退避、网络和服务端错误 |
| 数据迁移 | unrecoverable migration failure | 0 known data loss | 阻断发布，进入事故流程 |
| 可观测性 | error logs with trace_id | > 99% | 禁止合并无 trace 的新 API |
| 隐私 | PII raw log count | 0 | 立即修复并执行日志清理策略 |

错误预算规则:

1. Crash-free 低于目标，停止新功能发布，直到恢复到目标并完成复盘。
2. 核心路径 P95/P99 回退超过 10%，阻塞发布，除非有 ADR 接受该代价。
3. 数据迁移出现不可恢复错误，优先级高于所有功能开发。
4. 可观测性缺失导致问题无法定位，也算工程事故。
5. 连续两个版本触发同类 SLO 失败，必须更新手册、测试或 CI 门禁。

SLO 分级:

| 阶段 | 要求 |
|------|------|
| 原型 | 只记录本地指标，不阻塞迭代 |
| 内测 | 核心路径必须有 baseline，严重回退阻塞发版 |
| 商业发布 | SLO 进入发布门禁，错误预算消耗需复盘 |
| 规模化 | SLO 与告警、仪表盘、值班/响应流程绑定 |

### 13.5 Snapshot + WAL

Snapshot + WAL 是高级能力，不是 MVP 默认能力。不要承诺固定“50ms 启动”，应以真机测量决定是否启用。

适用条件:

1. App 启动需要重建大量内存状态。
2. 从数据库全量重建超过启动预算。
3. 内存状态可稳定序列化和版本化。
4. 有快照损坏检测与回退到 DB 重建的路径。

风险:

1. 快照格式演进复杂。
2. 加密快照会增加启动成本。
3. WAL 回放失败必须可恢复。
4. 需要额外测试崩溃中断场景。

---

## 14. 安全、隐私与合规

### 14.1 安全基线

1. Release 禁止 debug signing。
2. 敏感数据加密存储。
3. 密钥存平台 Keystore/Keychain。
4. 日志禁止 PII 原文。
5. 网络默认 TLS，敏感场景评估证书 pinning。
6. 依赖扫描进入 CI。
7. SBOM 随 release 归档。
8. 权限最小化。
9. 崩溃上报默认脱敏。
10. 调试后门禁止进入 release。

### 14.2 高安全场景加固

启用条件: 金融、医疗、企业机密、用户私密文本、大规模商业数据。

可选加固:

1. SQLCipher。
2. Rust `zeroize` 清理敏感内存。
3. Certificate pinning。
4. 本地二次加密。
5. Root/jailbreak 风险提示。
6. 数据导出加密。

加固会增加恢复难度和调试成本，必须配套备份、密钥恢复和用户支持策略。

### 14.3 隐私合规清单

1. 明确收集哪些数据。
2. 明确为什么收集。
3. 明确保留多久。
4. 支持导出用户数据。
5. 支持删除用户数据。
6. 支持关闭非必要 analytics。
7. iOS 隐私清单与 App Store 隐私标签一致。
8. Android Data Safety 与真实行为一致。
9. 涉及广告追踪时遵守 ATT / 广告 ID 规则。
10. 面向海外用户时评估 GDPR / CCPA。

### 14.4 数据生命周期总表

每类数据都必须知道从创建到删除的完整路径。隐私不是“加密一下”，而是生命周期治理。

| 数据类型 | 创建 | 存储 | 加密 | 上传 | 保留 | 导出 | 删除 |
|----------|------|------|------|------|------|------|------|
| UI 临时输入 | Flutter | 内存 / 页面 state | 否 | 否 | 页面生命周期 | 否 | 页面销毁 |
| UI 偏好 | Flutter 或 Rust | shared_preferences / 轻量 KV | 视敏感度 | 默认否 | 用户保留期 | 可选 | 用户重置或卸载 |
| 业务主数据 | Rust | SQLite | 视等级 | 可选同步 | 用户保留期 | 是 | 是 |
| 用户私密文本 | Rust | SQLite / SQLCipher | 建议加密 | 默认不上传 | 用户控制 | 是 | 是 |
| token / refresh token | 平台能力 | Keystore / Keychain | 是 | 否 | 会话或刷新周期 | 否 | 登出/吊销 |
| 加密密钥 | 平台能力 | Keystore / Keychain | 硬件或系统保护 | 否 | 账户生命周期 | 否 | 账户删除/重置 |
| 同步队列 | Rust | SQLite | 视 payload | 是 | 完成或过期前 | 通常否 | 成功/取消/过期 |
| 崩溃报告 | SDK / app | 本地缓冲 + 远端 | 脱敏 | 是 | 平台策略 | 按合规 | 按合规 |
| 结构化日志 | Flutter / Rust | 本地 / 远端 | 脱敏 | 采样上传 | 短周期 | 通常否 | 到期清理 |
| 导出文件 | Rust file service | App 沙箱 / 用户选择路径 | 可选加密 | 用户决定 | 用户控制 | 本身就是导出 | 用户删除 |
| 图片/音频缓存 | Flutter 或 Rust | cache dir | 通常否 | 否 | 可清理 | 否 | 缓存淘汰 |
| AI/向量索引 | Rust | SQLite / vector store | 视源数据 | 默认否 | 随源数据 | 可选 | 源数据删除时级联 |

生命周期规则:

1. 删除业务主数据时，read model、搜索索引、向量索引、缓存和同步队列必须级联处理。
2. 用户请求导出时，导出的必须是可解释数据，不是内部数据库裸文件，除非明确设计为完整备份。
3. 日志和崩溃报告默认不进入用户导出，但必须符合隐私政策和平台规则。
4. Token、密钥、会话数据不允许进入普通导出。
5. 任何新数据类型进入项目，都必须补进生命周期表。

### 14.5 威胁模型

每个发布版本至少维护一个轻量威胁模型。

| 资产 | 攻击面 | 防护 |
|------|--------|------|
| 用户文本/日记 | 本地 DB、备份、日志、崩溃上报 | 加密、脱敏、导出删除 |
| token/密钥 | Dart 内存、日志、反编译、剪贴板 | Keystore/Keychain、最小暴露、轮换 |
| FFI 边界 | 非法参数、生命周期错误、panic | DTO 校验、Result、RustOpaque 释放 |
| 网络请求 | 中间人、重放、弱网重复提交 | TLS、幂等键、证书 pinning 评估 |
| 深链/分享入口 | 参数注入、越权打开 | schema 校验、权限检查 |
| 本地文件 | 路径穿越、外部存储泄漏 | 沙箱路径、扩展名白名单 |

安全决策规则:

1. 客户端不能保存真正的长期服务端秘密。
2. API key 只能视为标识，不视为秘密。
3. 所有深链、通知、分享入口都必须重新校验权限和参数。
4. 剪贴板、截图、系统分享属于隐私边界，敏感内容默认最小暴露。
5. Root/jailbreak、反调试、混淆只能提高门槛，不能替代服务端校验和数据加密。

### 14.6 安全事件响应

发现安全问题后按以下流程处理:

1. 评级: P0 数据泄漏/账户接管，P1 权限绕过/密钥泄漏，P2 局部隐私风险，P3 低风险配置问题。
2. 止血: 关闭功能、吊销密钥、服务端拦截、暂停发布。
3. 修复: 最小补丁优先，避免混入无关重构。
4. 验证: 回归测试、安全复测、日志确认。
5. 通知: 按合规要求通知用户、平台或合作方。
6. 复盘: 记录根因、时间线、补偿措施和长期修复。

---

## 15. 可观测性

### 15.1 三层观测

1. Flutter: 帧时间、首帧、交互延迟、页面、错误、内存、GC。
2. FFI: API 名、耗时、参数规模、结果、错误码。
3. Rust: tracing span、DB 查询、锁等待、队列长度、同步状态。

### 15.2 工具分级

| 阶段 | 推荐 |
|------|------|
| MVP | 本地日志 + DevTools + Rust tracing |
| 产品化 | Sentry/Crashlytics + 自定义 FFI timing |
| 规模化 | OpenTelemetry + dashboard |
| 企业级 | Jaeger/Grafana + 采样策略 + 告警 |

OpenTelemetry 很强，但独立开发者不应默认上。先从轻量、能定位问题的指标开始。

### 15.3 Trace 透传

跨 Flutter、FFI、Rust 的调用必须保留同一个 trace 上下文。

1. Dart Controller 创建 `trace_id` 和 `action_id`。
2. FFI DTO 带上 `TraceContext`。
3. Rust API 入口创建 span: `ffi.api_name`。
4. DB、网络、队列、同步任务作为子 span。
5. 错误日志必须包含 trace_id、action_id、error_code。

```rust
pub struct TraceContext {
  pub trace_id: String,
  pub action_id: Option<i64>,
  pub screen: String,
}
```

### 15.4 结构化日志字段

| 字段 | 说明 |
|------|------|
| ts | UTC 时间 |
| level | trace/debug/info/warn/error |
| trace_id | 跨层追踪 ID |
| action_id | 用户动作 ID |
| feature | 功能域 |
| screen | 页面 |
| ffi_api | FFI API 名 |
| duration_us | 耗时 |
| input_size | 参数规模，不含 PII 原文 |
| error_code | 稳定错误码 |
| retryable | 是否可重试 |

采样规则:

1. crash / fatal / security error: 100%。
2. 核心写路径错误: 100%。
3. 核心成功路径: 1% 到 10%，按阶段调整。
4. 高频 UI 事件: 默认不上报或极低采样。
5. 本地 debug 日志可以详细，release 日志必须脱敏。

### 15.5 Tracing 桥接实操

§15.3 说了“trace 透传”，本节回答具体怎么实现。目标是让 Rust `tracing` 输出能被 Flutter / logcat / Sentry 统一看到，且能按 `trace_id` 关联。

#### 15.5.1 Rust 侧：统一 subscriber

```rust
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,rust_lib_fragments=debug"));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .json();   // 默认 JSON，便于上游采集

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(crash_reporter_layer())   // 可选：桥接 Sentry
        .with(platform_log_layer())     // logcat / os_log
        .init();
}
```

#### 15.5.2 桥接到平台日志

| 平台 | 推荐后端 | crate |
|------|----------|-------|
| Android | logcat | `tracing-android` 或手写 layer 调 `__android_log_print` |
| iOS | os_log / Console.app | `tracing-oslog` 或手写 layer 调 `os_log` |
| Sentry | breadcrumbs + events | `sentry-tracing` |
| 开发期 Flutter | stdout / DevTools | FRB `setup_default_user_utils()` 默认收到 `print()` |

#### 15.5.3 跨边界 trace context

```rust
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,        // ULID / UUIDv7
    pub action_id: Option<i64>,
    pub screen: String,
}

pub async fn load_fragment_detail(
    ctx: TraceContext,
    id: String,
) -> anyhow::Result<FragmentDetailDto> {
    let span = tracing::info_span!(
        "ffi.load_fragment_detail",
        trace_id = %ctx.trace_id,
        action_id = ctx.action_id,
        screen = %ctx.screen,
    );
    async move { /* ... */ }.instrument(span).await
}
```

Dart 侧在 Controller 生成 `trace_id`，随 DTO 送入；任何错误上报、崩溃、用户反馈都能通过同一个 `trace_id` 在 Flutter / FFI / Rust / DB 四层日志中检索。

#### 15.5.4 采样与背压

1. Rust `EnvFilter` 启动期可调，不重启调级用 `reload::Handle` 或远程 config。
2. 高频 span 默认 `level=trace`，生产过滤掉；例如渲染帧内多次调用不要用 `info!`。
3. 如果 bridge layer 本身阶段性卡顿，必须丢弃老日志而不是阻塞业务线程。

---

## 16. 测试体系

### 16.1 测试分层

| 类型 | 目标 |
|------|------|
| Rust unit | 领域规则、状态机、算法 |
| Rust property | 不变量、合并、排序、冲突处理 |
| Dart unit/widget | UI 状态、交互、错误展示 |
| FFI integration | 类型转换、生命周期、真实调用 |
| E2E | 用户主路径 |
| Performance | 启动、滚动、批量数据、内存 |

默认投入比例:

1. Rust unit/property tests: 40%。
2. Dart unit/widget tests: 25%。
3. Integration tests: 20%。
4. Manual exploratory tests: 10%。
5. Performance regression tests: 5%。

### 16.2 必测清单

1. Rust 领域规则。
2. 数据迁移。
3. FFI 契约和错误映射。
4. Stream 取消。
5. RustOpaque dispose。
6. 离线写入与同步恢复。
7. 分页边界。
8. 大数据导入导出。
9. 启动路径。
10. 用户数据导出和删除。

### 16.3 Property-Based Testing

启用条件:

1. 状态机复杂。
2. 合并/同步逻辑复杂。
3. 输入组合爆炸。
4. 手写案例无法覆盖边界。

不需要为简单 CRUD 强上 proptest。

### 16.4 FFI Mock 与集成测试范式

本节是 §16.1 “FFI integration” 和 §16.2 必测清单的落地指南，解决实际写代码时“Dart 测试怎么避开 Rust”与“集成测试怎么跑真 Rust”两个问题。

#### 16.4.1 三种测试层次

| 层次 | 是否加载 native lib | 运行环境 | 适用场景 |
|------|---------------------|----------|---------|
| 纯 Dart unit / widget test | 否（mock） | `flutter test` (Dart VM) | UI 状态、Provider 逻辑、错误映射 |
| Rust unit / property test | 否（不走 FFI） | `cargo test` | domain / application / infra 内部 |
| FFI integration test | 是（真加载） | `flutter test integration_test/` | 边界序列化、RustOpaque 生命周期、真实错误传递 |

默认原则：性价比低的测试走上面两层；FFI integration test 只覆盖“只有真调才能发现”的问题。

#### 16.4.2 Dart 单测：使用 `RustLib.initMock`

FRB 生成的 [`RustLib`](lib/src/rust/frb_generated.dart) 提供了 `initMock`，不加载 native lib、完全由 mock api 接管：

```dart
// test/fragments_provider_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:fragments/src/rust/frb_generated.dart';
import 'package:mocktail/mocktail.dart';

class _MockRustLibApi extends Mock implements RustLibApi {}

void main() {
  late _MockRustLibApi api;

  setUp(() {
    api = _MockRustLibApi();
    RustLib.initMock(api: api);
  });

  test('buildHomeView 返回后 Provider 同步 fadeLevel', () async {
    when(() => api.crateApiViewBuildHomeView(
          fragments: any(named: 'fragments'),
          recoveries: any(named: 'recoveries'),
          nowMs: any(named: 'nowMs'),
        )).thenReturn(HomeViewDto(
          schemaVersion: 1,
          fragments: [FragmentViewDto(id: 'a', fadeLevel: 0.5)],
          growthScore: 0.8,
        ));

    final provider = FragmentsProvider();
    await provider.load();

    expect(provider.fragments.first.fadeLevel, 0.5);
    expect(provider.growthScore, 0.8);
  });
}
```

规则：

1. **只 mock 生成的 `RustLibApi`**，不要 mock `RustBackend` 门面；门面本身必须被测覆盖。
2. 在每个 `setUp` 中调 `initMock`；不要跨测试复用 RustLib 实例状态。
3. mock 返回的 DTO 必须填写正确 `schemaVersion`，否则与生产路径不一致。
4. mock 不能代替集成测试：它证明的是 Dart 逻辑，不是跨语言契约。

#### 16.4.3 Rust 单测：避开 FFI

Rust 侧测试默认不走 FRB。`api/*.rs` 入口很薄（仅 DTO 转换 + tracing），重点测 `application/` 与 `domain/`：

```rust
// rust/src/domain/fade.rs 内嵌测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_level_decays_monotonically() { /* ... */ }

    #[test]
    fn record_recovery_advances_outburst_to_recovery() { /* ... */ }
}
```

仅在必要时（如验证 schema 校验错误代码）才写 `api` 层测试，调用手写函数但不经 FFI。

#### 16.4.4 集成测试：在设备上跑真 FFI

集成测试位于 [`integration_test/`](integration_test)，使用 `integration_test` 包，必须在真机或模拟器上运行：

```dart
// integration_test/ffi_smoke_test.dart
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:fragments/services/rust_backend.dart';
import 'package:fragments/src/rust/api/dto.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async {
    await RustBackend.init(); // 真加载 native lib
  });

  testWidgets('schema version 与 Rust 一致', (_) async {
    expect(supportedFragmentSchemaVersion(), RustBackend.expectedFragmentSchema);
    expect(supportedRecoverySchemaVersion(), RustBackend.expectedRecoverySchema);
  });

  testWidgets('buildHomeView 错误路径返回结构化异常', (_) async {
    final bad = FragmentDto(
      schemaVersion: 999, // 不支持的 schema
      id: 'x', createdAtMs: PlatformInt64Util.from(0),
      intensity: 5, fadePeriodDays: 270, stage: 'outburst',
    );
    expect(
      () => buildHomeView(fragments: [bad], recoveries: const [], nowMs: PlatformInt64Util.from(0)),
      throwsA(isA<AnyhowException>()),
    );
  });
}
```

规则：

1. **必测项** （对应 §16.2）：schema 版本调齐、错误传递、RustOpaque dispose 后访问、Stream 取消后不再推送、大 batch DTO 序列化。
2. 集成测试跑一次不贵，但不能取代单测。不要把 Provider 业务逻辑都塞到 integration test。
3. CI 至少跑一个 Android emulator 集成测试：如果只跑 unit，FFI 序列化问题会漏。
4. 集成测试中的错误断言必须接纳 Dart 侧看到的真实异常类型，不要在门面里护宇成 `Exception`。

#### 16.4.5 判例：Dart fallback 掩盖 Rust 初始化失败

此场景在 [CASE-0003](#case-0003-dart-fallback-掩盖-rust-初始化失败) 已记录。潜规则：

1. 生产代码在 `RustLib.init` 失败后 **fail-fast**，不提供 Dart fallback。
2. 如果产品设计确实需要 fallback（如调试面板），必须是可观测的 fallback——警告横幅 + 上报事件 + 不进入核心路径。
3. 测试环境不准在 `initMock` 之外静默降级为假实现。

---

## 17. 产品质量: i18n、a11y 与设计系统

### 17.1 i18n

1. 所有用户可见文本必须走本地化系统。
2. 文案不写死在 Widget 中。
3. 日期、数字、货币、复数规则必须本地化。
4. UI 需支持文本变长 30% 不崩。
5. 不同语言截图纳入发布检查。

### 17.2 a11y

1. 可点击区域至少 44x44 逻辑像素。
2. 所有图标按钮有语义标签。
3. 文本对比度符合 WCAG AA。
4. 支持系统字体缩放。
5. 关键流程可通过屏幕阅读器理解。

### 17.3 设计系统

1. 颜色、字体、间距、圆角、阴影统一 token 化。
2. 常用组件抽为 shared widgets。
3. 动效有统一时长和曲线。
4. 空状态、错误状态、加载状态有统一模板。
5. 设计系统不为炫技服务，只为一致性和速度服务。

### 17.4 UX 状态机

每个核心页面至少显式建模以下状态:

```text
initial -> loading -> content
loading -> empty
loading -> error
content -> refreshing -> content
content -> offline_stale
content -> partial_error
content -> pending_mutation
pending_mutation -> content | rollback
```

状态规则:

1. Loading 只用于首次无内容加载；已有内容刷新用 refreshing，不清空页面。
2. Empty 是成功状态，不是错误状态。
3. Offline stale 要展示最后更新时间和同步状态。
4. Partial error 不应摧毁整屏内容，例如列表加载成功但统计失败。
5. Pending mutation 必须有可见或可感知反馈。
6. Rollback 必须说明原因，并提供重试或撤销后的稳定状态。

统一组件契约:

| 状态 | 组件 | 要求 |
|------|------|------|
| Loading | skeleton / progress | 不闪烁，不阻塞返回 |
| Empty | empty state | 给下一步动作 |
| Error | inline error / full error | 按严重度选择，不泄漏技术细节 |
| Offline | offline banner | 不遮挡主要操作 |
| Syncing | small status | 不打断用户输入 |
| Conflict | conflict resolver | 展示差异和选择后果 |

### 17.5 深链、通知与冷启动

1. 深链、通知、分享入口不得绕过 Rust 初始化和权限校验。
2. 冷启动路径必须区分: 首帧、Rust ready、首屏数据 ready、目标路由 ready。
3. 如果目标数据未同步，先进入可解释的 pending/empty/offline 状态。
4. 深链参数必须 schema 校验，非法参数进入安全的默认页。
5. 通知点击导致的写操作必须二次确认，不能直接执行破坏性命令。

### 17.6 i18n 发布检查清单

1. 所有用户可见文本走本地化系统，不在 Widget 中硬编码。
2. 日期、时间、数字、货币、百分比使用 locale-aware formatter。
3. 不拼接语法敏感字符串，例如 `"删除 " + name`。
4. 中文、英文至少各跑一轮核心路径截图检查。
5. 长文本语言下按钮、标题、标签不溢出、不遮挡。
6. 空态、错误态、权限说明、同步状态都已本地化。
7. App 名称、隐私文案、权限弹窗文案与商店配置一致。
8. 暂不支持 RTL 时写入 `EXCEPTIONS.md`，并说明原因和重新评估条件。

### 17.7 a11y 发布检查清单

1. 所有 icon button 有语义标签。
2. 可点击区域至少 44x44 逻辑像素。
3. 字体缩放到 200% 时核心流程仍可操作。
4. 文本和关键 UI 对比度满足 WCAG AA。
5. 错误信息能被屏幕阅读器读懂。
6. 表单字段有 label、hint 和错误说明。
7. 关键状态不只依赖颜色表达，也有文字、图标或语义。
8. 动效遵守系统 reduce motion 或提供降级路径。
9. 页面焦点顺序符合视觉和操作顺序。
10. 离线、同步中、冲突、失败状态都有可访问表达。

### 17.8 隐私发布检查清单

1. 新增字段是否属于 PII、Sensitive 或 Secret。
2. 新增字段是否进入日志、崩溃报告、analytics 或远端同步。
3. 新增字段是否进入用户导出。
4. 用户删除数据时，新字段是否级联删除。
5. App Store 隐私标签和 Android Data Safety 是否需要更新。
6. 权限申请文案是否与真实用途一致。
7. 非必要 analytics 是否可关闭。
8. 调试日志、测试账号、后门开关是否被 release 排除。
9. 截图、剪贴板、分享、通知是否暴露敏感内容。
10. 第三方 SDK 是否新增数据收集行为。

---

## 18. 工程效能与仓库治理

### 18.1 Seam Pattern

Dart 与 Rust 之间保留一层抽象 seam:

1. Flutter 开发时可注入 mock backend。
2. Rust 开发时可用 CLI / unit test 独立验证。
3. FFI 联调集中在集成测试和关键路径。

Seam 的目标不是增加抽象，而是保持 Flutter 迭代速度，同时不牺牲真实集成测试。

### 18.2 生成代码纪律

1. FRB 生成代码不手改。
2. Rust API 变更必须重新生成并成对提交。
3. 生成命令脚本化。
4. CI 验证生成代码是否过期。

### 18.3 标准目录

下表是 Flutter + Rust + cargokit 项目的推荐布局（与本项目实际一致）。§6.7.1 列出了生成产物的边界，本表补充其它默认目录。

```text
app_root/
  README.md
  Flutter_Rust工程手册.md
  EXCEPTIONS.md
  flutter_rust_bridge.yaml          # FRB 生成器配置
  rust-toolchain.toml               # 固定 Rust toolchain
  doctor.ps1                        # Windows 环境检查（macOS 提供 shell 等价）
  docs/
    adr/                            # 架构决策
    cases/                          # 判例
    runbooks/                       # 事故响应手册
  lib/                              # Flutter / Dart 源码
    main.dart
    app.dart
    pages/  state/  services/  data/  models/  widgets/  theme/  i18n/  utils/
    src/rust/                       # FRB 生成产物（不手改）
  rust/                             # Rust 业务 crate
    Cargo.toml
    benches/                        # criterion 基准
    migrations/                     # SQL / schema
    src/
      lib.rs
      frb_generated.rs              # FRB 生成产物
      api/                          # FFI 入口（rust_input 根）
      application/                  # 用例编排、事务边界
      domain/                       # 纯业务规则
      infra/                        # SQLite / 加密 / 网络 / 文件
  rust_builder/                     # cargokit 提供的 Flutter plugin 包
    pubspec.yaml                    # name: rust_lib_<crate_name>
    android/  ios/  cargokit/
  android/  ios/                    # 平台工程
  test/  integration_test/
  .github/workflows/
```

说明:

1. **不要**在 `rust_builder/` 内写任何业务代码；它只是 Flutter 插件包装，负责调 cargokit 构建 `rust/`。
2. cargokit 的 `rust_builder/cargokit/` 是上游镜像，升级需按上游说明同步，不手改。
3. `lib/services/rust_backend.dart` 是 Dart 侧手写门面，隐藏生成层、集中做 DTO 转换与 schema 校验；UI / Provider 只调门面。

### 18.4 本地命令标准化

每个项目应提供统一命令:

```text
doctor
gen
analyze
test
bench
run-android
build-release
```

Windows 可用 PowerShell 脚本、melos scripts 或等价方案实现。

### 18.5 CI 标准

CI 至少包含:

1. Flutter analyze。
2. Flutter unit/widget tests。
3. Rust fmt。
4. Rust clippy -D warnings。
5. Rust tests。
6. 依赖安全扫描。
7. Android build。
8. 关键 integration test。

### 18.6 Lint 与质量门禁

Dart 最小规则建议:

1. 禁止隐式 dynamic 调用。
2. 避免未处理 Future。
3. 用户可见文案禁止硬编码在 Widget。
4. Provider / Controller 命名表达 feature 和职责。
5. 测试名表达行为。

Rust 最小规则建议:

1. `cargo fmt` 必须通过。
2. `cargo clippy -- -D warnings` 必须通过。
3. 业务 crate 禁止生产路径裸 `unwrap()` / `expect()`。
4. 新增 unsafe 必须有 ADR 或安全注释。
5. public API 必须表达错误语义。

CI 缓存策略:

1. 缓存 pub packages。
2. 缓存 Gradle。
3. 缓存 cargo registry 和 git dependencies。
4. Rust target 缓存要按 profile、target triple、Cargo.lock hash 区分。
5. 生成代码检查必须在缓存之后运行，避免缓存掩盖过期产物。

### 18.7 Onboarding 清单

新机器从零跑通项目必须有以下步骤:

1. 安装 Flutter SDK 并通过 `flutter doctor`。
2. 安装 Rust toolchain、Android targets、必要 linker。
3. 安装 Android Studio / Xcode，并确认模拟器或真机可用。
4. 拉取依赖: Flutter packages、Cargo crates、Gradle。
5. 执行代码生成。
6. 运行 Flutter analyze、Rust test、Dart tests。
7. 启动 debug App。
8. 运行一次 profile 启动检查。
9. 验证 FRB 真实调用，不允许只跑 Dart fallback。

### 18.8 风险登记册与事故复盘

风险登记册模板:

```markdown
| ID | 风险 | 概率 | 影响 | 触发信号 | 缓解措施 | Owner | 状态 |
|----|------|------|------|----------|----------|-------|------|
| R-001 | Rust 构建链不稳定 | 中 | 高 | CI Android build 失败 | 固定 NDK/Rust target | me | open |
```

事故复盘模板:

```markdown
# POST-0001: 发布后同步队列重复提交

- 日期:
- 影响:
- 时间线:
- 根因:
- 触发条件:
- 为什么测试没有发现:
- 修复:
- 长期措施:
- 负责人:
```

规则:

1. 事故复盘不追责个人，只追责系统缺陷。
2. 复盘必须产生测试、监控或流程改进。
3. 同类事故第二次出现，说明第一次复盘失败。

### 18.9 可执行模板最小集

手册里的规则最终要落到工具。建议维护以下模板，放在 `docs/templates/` 或项目脚本目录中。若仓库尚未创建这些文件，本节作为创建模板的标准来源。

| 模板 | 目的 | 触发位置 |
|------|------|----------|
| `analysis_options.strict.yaml` | Dart 静态规则 | 本地 analyze / CI |
| `clippy.toml` | Rust lint 参数 | 本地 clippy / CI |
| `cargo-deny.toml` | license、漏洞、重复依赖 | CI / 依赖升级 |
| `rust-toolchain.toml` | 固定 Rust 工具链 | 本地 / CI |
| `lefthook.yml` | pre-commit / pre-push 门禁 | 本地提交 |
| `github-actions-flutter-rust.yml` | 标准 CI 流水线 | PR / main |
| `doctor.ps1` | Windows 本地环境检查 | onboarding / 故障排查 |

最小 Dart lint 片段:

```yaml
include: package:flutter_lints/flutter.yaml

analyzer:
  language:
    strict-casts: true
    strict-inference: true
    strict-raw-types: true
  errors:
    unused_import: error
    dead_code: error

linter:
  rules:
    avoid_dynamic_calls: true
    avoid_print: true
    cancel_subscriptions: true
    close_sinks: true
    discarded_futures: true
    unawaited_futures: true
    prefer_final_locals: true
    require_trailing_commas: true
```

最小 Rust 工具链模板:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = [
  "aarch64-linux-android",
  "armv7-linux-androideabi",
  "x86_64-linux-android",
  "aarch64-apple-ios",
  "x86_64-apple-ios",
]
```

最小 clippy 策略:

```toml
avoid-breaking-exported-api = false
too-many-arguments-threshold = 8
type-complexity-threshold = 250
```

CI 中必须额外显式执行:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
flutter analyze
flutter test
```

最小 cargo-deny 策略:

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-3.0"]
copyleft = "deny"

[bans]
multiple-versions = "warn"
wildcards = "deny"
```

最小 lefthook 策略:

```yaml
pre-commit:
  parallel: true
  commands:
    flutter-analyze:
      run: flutter analyze
    rust-fmt:
      run: cargo fmt --check
    rust-clippy:
      run: cargo clippy --all-targets --all-features -- -D warnings

pre-push:
  commands:
    flutter-test:
      run: flutter test
    rust-test:
      run: cargo test --all
```

最小 GitHub Actions 流程骨架:

```yaml
name: ci

on:
  pull_request:
  push:
    branches: [main]

jobs:
  flutter-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          channel: stable
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: flutter pub get
      - run: flutter analyze
      - run: flutter test
      - run: cargo fmt --check
        working-directory: rust
      - run: cargo clippy --all-targets --all-features -- -D warnings
        working-directory: rust
      - run: cargo test --all
        working-directory: rust
```

模板治理规则:

1. 模板是起点，不是一次性复制后遗忘。
2. 每次手册新增硬规则，必须检查是否能加入 lint、CI、doctor 或 checklist。
3. 本地 hook 不能替代 CI，CI 才是发布事实。
4. 慢检查放 pre-push 或 CI，避免 pre-commit 过慢导致被绕过。
5. Windows 项目必须提供 PowerShell 版本 doctor，不能只给 Unix shell。

### 18.10 Doctor 脚本检查项

`doctor` 命令至少检查:

1. Flutter SDK 版本和 channel。
2. Dart SDK 版本。
3. Rust toolchain、host、installed targets。
4. Android SDK、NDK、Java、Gradle 可用性。
5. iOS 环境，若在 macOS。
6. FRB codegen 是否可执行。
7. `flutter pub get` 是否成功。
8. Cargo 依赖是否可解析。
9. 生成代码是否过期。
10. 是否能执行一次真实 Rust FFI smoke test。

doctor 输出规则:

1. 每项输出 OK / WARN / FAIL。
2. FAIL 必须给出下一步命令或文档位置。
3. 不输出密钥、token、完整用户路径或隐私信息。
4. doctor 成功不代表项目可发布，只代表环境可工作。

---

## 19. 依赖治理

### 19.1 引入依赖五维评估

每项 1 到 5 分:

1. 稳定性。
2. 可移除性。
3. 安全历史。
4. 维护活跃度。
5. 与架构边界匹配度。

总分 < 18 拒绝，18 到 22 观察，23 以上可引入。

### 19.2 更新策略

1. patch 可较快升级。
2. minor 先看 changelog 和 issue。
3. major 必须建升级分支。
4. Flutter、Rust、FRB 三类工具链升级不要混在一个提交。
5. 升级后必须跑 analyze、test、Android 真机/模拟器、至少一次 profile 启动。

### 19.3 锁版本策略

1. App 项目提交 lockfile。
2. Rust crate 关键依赖明确版本范围。
3. 生产发布记录依赖快照。
4. 安全漏洞升级优先级高于功能开发。

### 19.4 技术选型与反选型

技术选型必须能被未来的自己质疑。默认选择不是信仰，而是当前阶段下的最小可维护方案。

| 当前默认 | 为什么选 | 为什么暂不选其他 | 重新评估条件 |
|----------|----------|------------------|--------------|
| flutter_rust_bridge | 类型生成、async/stream 支持、独立开发维护成本低 | 手写 `dart:ffi` 粘合成本高；platform channel 不适合作为高频业务边界 | FRB 长期维护风险、需要极限手写 FFI 优化 |
| Riverpod | selector、测试、组合性、Controller 分层清晰 | Provider 在复杂状态组合上弱；Bloc 样板较多 | 团队统一其他状态框架、Riverpod 维护风险 |
| SQLite | 事务、查询、索引、迁移、生态成熟 | Hive/Isar 适合部分对象存储，但复杂查询和迁移策略不同 | 数据模型明显不适合关系型或需要专用引擎 |
| SQLCipher | 高隐私结构化数据加密成熟 | 自研加密数据库风险高 | 安全需求降低或平台原生加密足够 |
| Tokio | Rust async 生态主流、库兼容性强 | async-std 生态相对弱；手写线程池成本高 | 依赖生态变化或 Tokio 成为瓶颈 |
| rusqlite / sqlx | 覆盖同步简单场景和异步复杂场景 | ORM 可能隐藏 SQL 和迁移成本 | 查询复杂度需要更强抽象且可被测试覆盖 |
| keyset pagination | 大列表稳定、性能可预测 | OFFSET 简单但大数据退化 | 仅小数据或后台一次性工具 |
| Sentry/Crashlytics 起步 | 独立开发者成本低、上线快 | OpenTelemetry 初期成本高 | 跨 Flutter/FFI/Rust 问题无法定位 |
| criterion | Rust benchmark 成熟 | 手写计时不稳定 | 需要端到端移动设备性能基准 |
| GitHub Actions / 等价 CI | 易接入、生态成熟 | 自建 CI 维护成本高 | 构建时长、缓存或合规要求超出托管能力 |

反选型规则:

1. “更流行”不是选型理由。
2. “以后可能用到”不是引入依赖理由。
3. 替换基础设施必须给迁移成本、回滚方案和双跑验证。
4. 同一层不要同时引入两个解决同一问题的主框架。
5. 依赖一旦进入核心路径，删除成本必须被当作长期成本计算。

### 19.5 环境与版本兼容矩阵

Flutter + Rust 项目的最大隐性风险之一是工具链漂移。所有关键版本必须被固定、记录和验证。

| 组件 | 固定位置 | 升级触发 | 必跑验证 | 备注 |
|------|----------|----------|----------|------|
| Flutter SDK | FVM / README / CI image | Flutter stable 修复、安全或平台要求 | `flutter doctor`、analyze、test、run | 不与 FRB/Rust major 混合升级 |
| Dart SDK | 随 Flutter | 随 Flutter | analyze、生成代码检查 | 注意 linter 行为变化 |
| Rust toolchain | `rust-toolchain.toml` | 安全、编译器 bug、依赖要求 | fmt、clippy、test、Android/iOS build | 记录 host 与 target |
| Cargo dependencies | `Cargo.lock` | 安全、bug、明确收益 | cargo test、cargo deny/audit | 核心 crate 升级单独提交 |
| FRB / codegen | `pubspec.yaml` + Cargo + 生成命令 | FRB bug、新能力、兼容需求 | gen、FFI integration、Android build | Dart/Rust/生成产物成对提交 |
| Android Gradle Plugin | Gradle files | Play 要求、安全或构建问题 | Android debug/release build | 与 Gradle wrapper 配套 |
| Android NDK | local.properties / CI | Rust native build、Play 要求 | Android release build | 记录 16KB page size 兼容性 |
| minSdk / targetSdk | Gradle | 商店要求、功能需求 | 安装、权限、后台任务测试 | targetSdk 升级需隐私权限复核 |
| iOS deployment target | Xcode / Podfile | App Store 要求、依赖要求 | archive、真机启动 | 记录 dSYM 上传流程 |
| Xcode / CocoaPods | README / CI | iOS 构建要求 | pod install、archive | macOS CI 要固定镜像 |
| SQLite / SQLCipher | Cargo / native config | 安全、迁移能力 | migration tests、加密打开测试 | 加密参数变更需备份策略 |
| Sentry/Crashlytics SDK | pubspec / native config | crash 上报 bug、安全 | crash smoke test、符号化验证 | release 前检查 dSYM/symbols |

升级批次规则:

1. Flutter、Rust、FRB、Android Gradle Plugin、Xcode 不在同一提交中混升。
2. 每次工具链升级必须先跑最小样例或当前 App 的真实 FFI 调用。
3. 版本升级 PR 必须说明升级原因、影响范围、回滚方式和验证结果。
4. CI 与本地版本不一致时，以 CI 为发布事实，本地必须对齐。
5. 新机器 onboarding 失败，优先更新环境矩阵和 doctor 脚本，而不是口头解释。

---

## 20. 发布、回滚、灾备与退役

### 20.1 发布流程

1. 冻结功能。
2. 跑全量质量门禁。
3. 跑 profile 性能检查。
4. 检查隐私与权限变更。
5. 生成 changelog。
6. 上传内测渠道。
7. 观察崩溃和核心路径指标。
8. 分阶段发布。

### 20.2 回滚策略

移动 App 无法像服务端一样瞬间回滚，因此必须有:

1. feature flag 或本地配置降级。
2. 兼容旧 schema 的数据迁移策略。
3. 线上错误开关。
4. 用户数据导出能力。

### 20.3 灾备与密钥管理

必须备份:

1. 代码仓库。
2. 签名证书 / keystore。
3. keystore 密码。
4. App Store / Play Console 账号恢复方式。
5. CI secrets 清单。
6. 数据库迁移历史。
7. 设计资源与商标字体授权。

备份规则:

1. 3-2-1 原则: 三份副本、两种介质、一份异地。
2. 密钥不能只存在本机。
3. 备份恢复流程每季度演练一次。
4. 任何密钥泄漏都必须轮换并记录。

### 20.4 App 退役协议

如果项目停止维护:

1. 明确停止维护日期。
2. 提供用户数据导出。
3. 停止不必要的数据收集。
4. 下线服务端接口前给迁移窗口。
5. 归档源码、签名、构建说明。

---

## 21. 法律、授权与资源

1. 所有第三方库 license 必须可用于商业发布。
2. 字体、图标、图片、音效必须有授权来源。
3. 开源组件 license 清单随 release 保存。
4. GPL/AGPL 类依赖默认禁止进入闭源商业 App，除非明确理解后果。
5. 产品名、图标、品牌需避免商标冲突。
6. 用户生成内容如涉及分享或云同步，需要内容政策与举报机制评估。

---

## 22. AI 辅助开发协议

1. AI 可以生成代码，但开发者必须读懂每一行。
2. AI 不能替代架构决策，只能提供候选方案。
3. AI 引入依赖必须走依赖评估。
4. AI 的性能优化建议必须用 benchmark 验证。
5. AI 不直接处理真实密钥、真实用户数据、未脱敏日志。
6. AI 修改安全敏感代码后必须人工复审。
7. AI 适合做脚手架、测试、文档、代码迁移、错误解释、方案对比。
8. AI 不适合独立决定隐私策略、加密方案、支付逻辑、账户安全。

---

## 23. 反模式黑名单

看到以下行为必须停下:

1. Widget 里写业务规则。
2. Dart 循环逐条调用 FFI。
3. 写后全量 reload。
4. 大列表一次性加载全表。
5. 跨 FFI 传 JSON 作为主协议。
6. Release 使用 debug signing。
7. 生产路径裸 unwrap。
8. 无界 channel。
9. Stream 不可取消。
10. 无 schema_version 的 DTO。
11. TODO 无到期日。
12. 隐私文案与真实行为不一致。
13. 依赖升级和功能改动混在一个提交。
14. 没有备份的数据库迁移。
15. UI 状态与业务状态双写。
16. 为了“未来复用”提前抽象三层以上。
17. 没有明确协作需求就引入 CRDT。
18. 没有生命周期隔离问题就引入 Actor。
19. 把服务端错误、SQL 错误或 Rust panic 文本直接展示给用户。
20. 深链和通知入口绕过权限校验。
21. 把 API key 当秘密放进客户端。
22. 用平均耗时证明性能优化。
23. 只在 debug 模式验证内存和 FFI 生命周期。
24. 用 `panic = "abort"` 后仍假设 panic 会变成 Dart 异常。
25. 没有 trace_id 的跨层问题定位。

---

## 24. 自检清单

### 24.1 每次提交只查三件事

1. 是否跨层或破坏边界。
2. 是否破坏测试或缺少必要测试。
3. 是否引入安全、隐私、数据风险。

### 24.2 每个功能完成前查十件事

1. 用例边界清楚吗。
2. FFI API 粗粒度吗。
3. 数据模型有 schema_version 吗。
4. 是否分页、批量、增量。
5. 错误是否可恢复、可解释。
6. 是否埋点。
7. 是否覆盖离线/弱网。
8. 是否支持 i18n/a11y 基线。
9. 是否通过 profile 真机检查。
10. 文档和 changelog 是否更新。

### 24.3 每个 FFI API 合并前查十件事

1. API 是否表达完整用例。
2. 是否选择了正确调用档位。
3. 是否有稳定 DTO 和 schema_version。
4. 是否有 trace context。
5. 是否有结构化错误 code。
6. 是否支持取消或说明无需取消。
7. 是否避免 Dart 循环调用。
8. 是否有参数规模上限。
9. 是否有测试覆盖成功、失败、取消。
10. 是否重新生成并验证生成代码未过期。

### 24.4 每次发布前查十件事

1. release signing 是否正确。
2. dSYM / Android symbols 是否归档或上传。
3. 隐私清单与真实行为是否一致。
4. crash reporting 是否可用且脱敏。
5. 数据迁移是否可恢复。
6. feature flag / 降级开关是否可用。
7. 依赖安全扫描是否通过。
8. 体积增长是否解释。
9. 核心路径性能是否未回退。
10. 用户数据导出/删除是否仍可用。

### 24.5 每次模板或 CI 变更前查十件事

1. 是否会显著增加本地提交耗时。
2. 是否在 Windows/macOS/Linux 行为一致。
3. 是否与当前 Flutter/Rust/FRB 版本兼容。
4. 是否会误拦截生成代码。
5. 是否会泄漏环境变量、token 或本地路径。
6. 是否有缓存键，避免 CI 无意义变慢。
7. 是否有失败时的修复提示。
8. 是否和 release 门禁一致。
9. 是否需要更新 onboarding 文档。
10. 是否需要记录到变更记录。

---

## 25. 技术债务台账模板

债务不是罪，隐形债务才是。

```markdown
| ID | 日期 | 类型 | 位置 | 原因 | 风险 | 偿还计划 | 到期日 | 状态 |
|----|------|------|------|------|------|----------|--------|------|
| TD-001 | 2026-05-03 | 架构 | fragments_provider | 临时保留 reload | 大数据卡顿 | 改 Stream | 2026-05-20 | open |
```

规则:

1. 每条债务必须有到期日。
2. 到期后必须决策: 偿还、延期、删除、接受。
3. 债务超过 10 条时停止新功能一天。

---

## 26. ADR 模板

```markdown
# ADR-0001: 选用 flutter_rust_bridge 作为 Flutter/Rust 桥接层

- 状态: Accepted
- 日期: 2026-05-03

## 背景

需要在 Flutter 中调用 Rust 业务内核，同时减少手写 FFI 成本。

## 选项

1. 直接 dart:ffi。
2. flutter_rust_bridge。
3. platform channel + 原生包装。

## 决策

选择 flutter_rust_bridge。

## 理由

1. 类型生成完整。
2. async/stream 支持成熟。
3. 适合独立开发者降低维护成本。

## 后果

1. 需要跟随 FRB 版本升级。
2. 生成代码必须成对提交。

## 重新评估条件

1. FRB 出现长期维护风险。
2. 项目需要极限手写 FFI 优化。
```

---

## 27. 判例库

### CASE-0001: 写后全量 reload

- 现象: 新增数据后列表整体刷新，数据量上升后卡顿。
- 根因: 写操作后执行全量 load，违反单向流和增量更新原则。
- 正确方案: Rust mutation 后发 FragmentEvent，Dart 侧增量合并。
- 教训: `await write(); await load();` 是架构异味。

### CASE-0002: Windows host 工具链影响 Android Rust 构建

- 现象: Android build 过程中出现 `link.exe not found`。
- 根因: rustup default-host 解析到 MSVC，cargokit 构建 Android 时链路错误。
- 修复: 设置 `rustup set default-host x86_64-pc-windows-gnu`，并安装 Android targets。
- 教训: Flutter + Rust 项目必须记录 host 工具链，不只记录 target。

### CASE-0003: Dart fallback 掩盖 Rust 初始化失败

- 现象: App 表面可运行，但 Rust 后端失败时被 Dart fallback 隐藏。
- 根因: fallback 让核心架构失真，测试无法暴露真实集成问题。
- 修复: Rust 初始化失败直接 fail fast，测试使用 FRB mock。
- 教训: 核心内核不可静默降级，除非降级本身是产品设计。

### CASE-0004: Sync FFI 中做 IO 导致掉帧

- 现象: 页面打开偶发卡顿，profile 看到 UI frame 超过预算。
- 根因: `#[frb(sync)]` 函数读取配置文件并解析 JSON。
- 修复: 改为 async API，启动阶段预热配置，UI 订阅 read model。
- 教训: sync FFI 只允许极轻纯函数。

### CASE-0005: RustOpaque 未释放导致内存上涨

- 现象: 反复进入详情页后 RSS 持续上涨。
- 根因: Dart provider 持有 RustOpaque，页面 dispose 未释放，GC 也不及时。
- 修复: owner 绑定路由生命周期，dispose 显式释放，Finalizer 只兜底。
- 教训: 跨语言对象生命周期必须显式设计。

### CASE-0006: 客户端自增 ID 破坏多端同步

- 现象: 多设备合并时本地 id 冲突，远端记录覆盖错误。
- 根因: 业务对象只使用 SQLite integer primary key。
- 修复: 增加 UUIDv7 global_id，本地 rowid 仅作数据库内部主键。
- 教训: 离线优先对象必须有客户端可生成的全局 ID。

### CASE-0007: 无 trace_id 导致跨层问题无法定位

- 现象: 用户反馈“保存慢”，Flutter、FFI、Rust、DB 日志无法关联。
- 根因: Dart action 没有 trace context，Rust span 与 UI 行为断开。
- 修复: Controller 生成 trace_id，经 FFI DTO 传入 Rust，所有日志带 trace_id。
- 教训: 可观测性必须从 API 设计开始。

---

## 28. SpendWhy / 碎片当前落地路线

### 28.1 当前阶段判断

SpendWhy / 碎片当前应按 M2 产品化阶段推进。目标不是立刻上 Actor、CRDT、ECS，而是先消灭架构基础债务。

### 28.2 当前指标基线

路线必须从事实出发。未测不是 0，而是风险。

| 指标 | 当前值 | 目标 | 获取方式 | 状态 |
|------|--------|------|----------|------|
| 冷启动 first_frame P95 | 未测 | < 1.5s | profile mode / Timeline | todo |
| 首屏 ready P95 | 未测 | < 2.5s | Timeline + app marker | todo |
| Rust init P95 | 未测 | 项目基线后设定 | tracing span | todo |
| async FFI P99 | 未测 | < 16ms | FFI timing / tracing | todo |
| sync FFI P99 | 未测 | < 100us | micro benchmark | todo |
| release arm64 体积 | 未测 | < 25MB | build output | todo |
| crash-free users | 未上线 | > 99.5% | Sentry/Crashlytics | todo |
| 写后全量 reload 数量 | 未统计 | 0 | code search / review | todo |
| 全表查询数量 | 未统计 | 0 个核心路径 | SQL audit / tracing | todo |
| 无 trace_id 错误日志比例 | 未测 | < 1% | log audit | todo |
| 数据迁移测试数量 | 未统计 | 每个迁移至少 1 个 | test report | todo |
| i18n 硬编码文案数量 | 未统计 | 0 个用户可见核心文案 | l10n audit | todo |

基线规则:

1. 第一轮优化前先补测，不允许凭感觉排序。
2. 当前值为“未测”的指标不能用于证明质量，只能用于标记风险。
3. 每个阶段结束时更新一次当前值和状态。
4. 如果指标采集成本过高，先记录采集方案和降级指标。

### 28.3 阶段 A: 立即消除高风险问题

1. Release 签名从 debug signing 改为正式 keystore 配置。
2. 列表写后 reload 改为 Stream 增量事件。
3. 查询改 keyset pagination。
4. 增加 FFI 调用耗时埋点。
5. 清理未使用依赖或落实用途。
6. README 从模板改成项目说明。
7. analysis_options 升级为更严格规则。

### 28.4 阶段 B: 架构对齐

1. 设计 Rust repository trait。
2. 持久层迁移到 Rust repository。
3. Dart 侧只保留 thin wrapper。
4. 引入 batch DTO 与列式数据结构。
5. fade/growth 建立 criterion benchmark。
6. 建立 `docs/adr` 与技术债台账。
7. 增加迁移测试。

### 28.5 阶段 C: 发布级质量

1. CI 矩阵完整化。
2. Sentry 或等价观测接入。
3. 隐私、权限、数据导出删除闭环。
4. i18n/a11y 基线检查。
5. 形成可复用模板仓库。

### 28.6 暂不建议做

1. CRDT: 当前没有多人/多设备协作刚需。
2. Actor: 当前状态域还不够复杂。
3. Snapshot + WAL: 先测启动瓶颈再决定。
4. ECS: 当前不是海量图节点应用。
5. 端侧 AI: 等产品核心闭环稳定后再评估。

---

## 29. 极端架构附录

这些不是默认方案，而是特殊问题的高级工具。

### 29.1 ECS + Canvas 裸绘

适用:

1. 屏幕上存在数万到十万级可交互节点。
2. Flutter Widget 树无法承受。
3. 业务更接近设计工具、图编辑器、游戏编辑器。

不适用:

1. 普通列表。
2. 表单应用。
3. 内容社区。

### 29.2 端侧 AI Native

适用:

1. 隐私要求高，不能上传原始数据。
2. 离线推理有产品价值。
3. 模型体积、耗电、发热可接受。
4. 有明确降级策略。

风险:

1. 包体巨大。
2. 低端设备不可用。
3. 发热耗电。
4. 模型更新困难。

### 29.3 WASM / Web 扩展

当前默认目标是 iOS / Android。Web/WASM 只有在产品明确需要时启用。

启用前必须评估:

1. FRB/Web 支持成熟度。
2. WASM 与 Dart JS 互操作成本。
3. SharedArrayBuffer 的安全 header 要求。
4. 移动 Web 性能是否达标。

---

## 30. 文档治理

1. 每个项目根目录保留一份统一手册。
2. 项目差异写 `EXCEPTIONS.md`。
3. 架构决策写 ADR。
4. 踩坑写判例。
5. 技术债写台账。
6. 每个项目里程碑后回顾一次本文档。
7. 如果某条规则在三个项目中都被违反，优先怀疑规则需要修订。

旧文档合并策略:

1. 《开发哲学.md》的执行标准并入本文的原则、分层、测试、发布、治理章节。
2. 《白皮书.md》的长期架构路线并入成熟度模型、极端架构附录和 SpendWhy 路线章节。
3. 原《白皮书.md》末尾宣发型内容中的绝对化表述已修正为可测量、可启用、可退出的工程标准。
4. 原始文件可保留为历史材料，正式执行以本文为准。

文档升级标准:

1. 只写原则不够，优先补决策树、反例、模板和检查项。
2. 每次事故后至少更新判例库、测试清单或反模式黑名单之一。
3. 每次引入新基础设施后补 ADR 和退出条件。
4. 每季度删除或降级已经不适用的规则。
5. 文档不追求厚，追求能减少下一次错误。

### 30.1 演进证据

顶级手册必须允许自己被现实修正。重要规则需要记录它如何产生、何时被验证、何时被推翻。

```markdown
| 规则 | 形成原因 | 被验证的案例 | 被挑战的案例 | 当前状态 |
|------|----------|--------------|--------------|----------|
| FFI 粗粒度 | 避免跨语言 getter 风暴 | CASE-0001 | 暂无 | active |
```

规则状态:

1. `active`: 当前默认执行。
2. `experimental`: 只在指定 feature 试行。
3. `deprecated`: 不再用于新代码，旧代码逐步迁移。
4. `rejected`: 已证明不适合本项目。

如果一条规则没有被现实验证过，它只能是建议，不能伪装成铁律。

---

## 31. 最终原则

Flutter 只承担体验复杂度，Rust 承担业务复杂度，FFI 承担稳定契约，指标承担事实判断，文档承担长期记忆。

成熟的架构不是一次性上满所有高级技术，而是在每个阶段只引入刚好足够的复杂度。能用 M2 解决的问题，不上 M4；能用批量 API 解决的问题，不上 Actor；能用服务端裁决解决的同步，不上 CRDT；能用 Widget 解决的渲染，不上 ECS。

强大的工程不是堆技术，而是知道每个技术何时值得付出代价。

---

## 附录 A. 术语表

按字母顺序排列。首次出现于本手册的全部术语都应在此补齐。

| 术语 | 全称 / 含义 | 本手册主要出处 |
|------|--------------|-------------------|
| **a11y** | Accessibility，无障碍访问。语义标签、对比度、焦点顺序、屏幕阅读器可读。 | §17.2, §17.7 |
| **ABI** | Application Binary Interface。Android 上指 `arm64-v8a` / `armeabi-v7a` / `x86_64` 等原生架构。 | §6.10 |
| **ADR** | Architecture Decision Record。记录架构决策的背景、选项、决定、后果。 | §2.x, §26, §30 |
| **AGP** | Android Gradle Plugin。 | §6.10, §19.5 |
| **Actor** | 独立生命周期与状态、通过消息交互的并发模型。 | §4.1.5, §10.4 |
| **AppError** | 本手册约定的跨边界错误枚举，带稳定 `code`。 | §11.2 |
| **anyhow** | Rust 错误上下文增强 crate，`anyhow::Result<T>` 适合业务边界返回。 | §6.8 |
| **cargokit** | Flutter 插件中集成 Rust 构建的工具链。本项目位于 [`rust_builder/cargokit/`](rust_builder/cargokit)。 | §6.10, §6.11, §18.3 |
| **CRDT** | Conflict-free Replicated Data Type。多节点离线编辑可自动合并的数据结构。 | §9.4 |
| **DTO** | Data Transfer Object。跨 FFI 契约快照，必带 `schema_version`。 | §6.5.7, §7.1, §6.9 |
| **ECS** | Entity-Component-System。适合海量节点的渲染 / 模拟架构。 | §4.1.5, §29.1 |
| **Error Budget** | 错误预算。SLO 允许被消耗的额度，超阈触发明确动作。 | §13.4 |
| **FFI** | Foreign Function Interface。跨语言调用边界，本手册特指 Dart ↔ Rust。 | 全文 |
| **FRB** | flutter_rust_bridge。本项目选用的 Dart↔Rust 生成器与运行时，当前 v2.12.0。 | §6.5–6.11 |
| **frb(init)** | FRB 提供的属性宏，标记 `RustLib.init` 时调用的入口。 | §6.8.6 |
| **frb(mirror)** | FRB 提供的属性宏，镜像第三方类型让其可被生成。 | §6.8.5 |
| **frb(opaque)** | FRB 属性宏，声明该类型以 RustOpaque 句柄跨边界传递。 | §6.8.4 |
| **frb(sync)** | FRB 属性宏，声明同步 FFI，只能用于极轻纯函数。 | §6.1, §6.8.1 |
| **FTS5** | SQLite 全文检索扩展。 | §7.2, §4.1.3 |
| **JNI** | Java Native Interface。Android 加载 Rust `.so` 的原生接口层。 | §6.10 |
| **keyset pagination** | 以游标 (`(created_ms, id)` 等) 分页，取代 OFFSET。 | §7.3, §4.2 |
| **LWW** | Last-Write-Wins。以最后一次写入为准的合并策略。 | §9.3 |
| **NDK** | Android Native Development Kit。 | §6.5.6, §6.10 |
| **OpenTelemetry** | 可观测性标准，跨 trace / metrics / logs。 | §15.2, §4.1.5 |
| **PII** | Personally Identifiable Information。个人可识别信息，默认不进日志。 | §13.4, §15.4 |
| **Read Model** | 为查询 / UI 优化的投影，可从事实源重建。 | §2.1.1, §5.5 |
| **RTL** | Right-To-Left。阿拉伯语 / 希伯来语等从右向左书写。 | §17.6 |
| **RustOpaque** | FRB 提供的跨语言句柄类型，Dart 侧持有 Rust 对象引用。 | §6.5.5, §6.8.4, §12.2 |
| **SBOM** | Software Bill of Materials。依赖清单。 | §14.1 |
| **schema_version** | DTO / migration 中的整型版本号，跨边界兼容检查依据。 | §6.9, §7.1 |
| **Seam** | Dart 与 Rust 之间保留的抽象层，便于 mock 与独立迭代。 | §18.1 |
| **Sink / StreamSink** | FRB 提供的 Rust→Dart 事件堆入口。 | §6.8.3 |
| **SLI / SLO** | Service Level Indicator / Objective。度量与发布契约。 | §13.4 |
| **SSE codec** | FRB 默认使用的跨边界二进制序列化协议。 | §6.5, [rust/src/frb_generated.rs](rust/src/frb_generated.rs) |
| **SQLCipher** | SQLite 的加密发行版。 | §7.2, §14.2 |
| **Tokio** | Rust 主流 async runtime。 | §6.5.3, §10.1 |
| **tracing** | Rust 结构化日志 / span crate，本手册默认选型。 | §15.5 |
| **UB** | Undefined Behavior。Rust unsafe 代码破坏不变量后的未定义行为，可造成崩溃、静默损坏、安全漏洞。 | §11.6 |
| **ULID / UUIDv7** | 可按时间排序的全局 ID。 | §7.5 |
| **WAL** | Write-Ahead Log。多用于 SQLite 并发 + Snapshot 高级场景。 | §7.4, §13.5 |
| **xcframework** | Apple 多架构打包格式，Rust iOS 静态库可装进去。 | §6.11 |
| **Zero-copy** | 不复制地跨 FFI 传递大二进制。本项目以 `ZeroCopyBuffer` 实现。 | §6.1, §12.1 |
| **miri** | Rust nightly 上的解释器，检测 unsafe 代码的 UB / 别名违反 / 未初始化读。 | §11.6.4 |
| **MethodChannel** | Flutter Dart↔原生代码的默认 RPC 通道。 | §6.12 |
| **PlatformException** | Platform Channel 调用失败时 Dart 侧收到的异常类型。 | §6.12.4 |
| **PlatformView** | Flutter 嵌入原生视图的机制，常与 Platform Channel 配套。 | §6.12 |

---

## 变更记录

- v4.7 (2026-05-11): 新增 §6.13 Flutter ↔ Rust 调用模式全景（cookbook）。1 张模式选择速查表 + 17 个场景模式：同步纯函数 / 异步业务用例 / Stream / 可取消长任务 / 进度+取消 / RustOpaque 句柄 / Zero-copy / 双向交互 / 事件总线 / 请求合并 / Keyset 分页+列式批量 / 重试+幂等 / Rust 调 Dart 回调 / 并发限流 / Isolate / 单例服务 / mock。每模式包含「何时用 / Rust 模板 / Dart 模板 / 陷阱」四要素，并以 §6.13.18 八问清单作为 PR 自检。同步更新目录。
- v4.6 (2026-05-11): 补齐三块中等优先级缺口。新增 §6.12 Platform Channels 与 FRB 共存指引（选型决策树、数据流模式、错误/Trace 对齐、反模式）；新增 §11.6 Unsafe Rust 边界（适用场景、SAFETY 注释纪律、不变量文档、miri/sanitizer/fuzz 验证手段、ADR 要求）；新增 §16.4 FFI Mock 与集成测试范式（三层测试职责、`RustLib.initMock` 样例、integration_test 样例、fail-fast 原则）。同步更新目录与附录 A 术语表（补 UB / miri / MethodChannel / PlatformException / PlatformView）。
- v4.5 (2026-05-11): 主要提升手册的「可检索性」与「可操作性」。新增 §0.1 角色化阅读指南、§0.2 完整目录；拆分 §6.7 / §6.8 / §6.9 / §6.10 / §6.11 补齐 FRB 代码生成工作流、常用代码模式（sync/async/Stream/RustOpaque/mirror/init）、FFI 演进与版本协议（含字段级变更矩阵与 deprecation 流程）、Android/iOS 平台构建细节；新增 §15.5 Tracing 桥接实操（含跨平台 layer、跨边界 trace context）；重写 §18.3 标准目录以反映 cargokit + rust_builder 事实布局；新增附录 A 术语表，覆盖 DTO / RustOpaque / FRB / SSE / WAL / CRDT / Actor / ECS / cargokit / xcframework 等 40+ 术语。
- v4.4 (2026-05-03): 补强可执行体系。新增 i18n、a11y、隐私发布检查清单，新增可执行模板最小集，包含 Dart lint、Rust toolchain、clippy、cargo-deny、lefthook 和 GitHub Actions 的最小配置片段，新增 doctor 脚本检查项和模板/CI 变更检查清单。
- v4.3 (2026-05-03): 补强工程契约层。新增 SLI/SLO/Error Budget、SLO 分级与错误预算规则，补充数据生命周期总表，新增环境与版本兼容矩阵及升级批次规则，并为 SpendWhy 增加当前指标基线表。调整 Snapshot/WAL、安全章节和 SpendWhy 路线小节编号。
- v4.2 (2026-05-03): 补强顶层思维框架。新增核心思维模型，包括事实源模型、用例模型、一致性窗口模型、复杂度预算模型和失败优先模型。新增架构取舍矩阵、技术选型与反选型、文档演进证据模板。统一 UI 偏好边界表述，修正本地写入流程对同步触发器的引用，调整错误章节编号，并补充“1 万条数据常驻内存”预算的适用范围说明。
- v4.1 (2026-05-03): 根据审稿补强为机制型手册。新增核心决策树、Rust 内部分层规范、Repository/事务边界、FRB 版本治理与 Isolate/Stream/RustOpaque/构建体积细节、跨边界类型规范、ID/时间/精度规则、数据库连接并发、服务端网络契约、错误到 UI 映射、威胁模型、安全事件响应、trace 透传、结构化日志字段、UX 状态机、深链通知冷启动、lint/CI 缓存、Onboarding、风险登记册、事故复盘、FFI/API/发布检查清单，并扩充反模式和判例库。
- v4.0 (2026-05-03): 合并《白皮书.md》与《开发哲学.md》为统一工程手册。去重 FFI、分层、状态、性能、安全、测试、发布和 SpendWhy 路线章节。修正过度绝对化表述，将“0 延迟”“锁定 120fps”“50ms 冷启动”“FFI panic 默认可靠捕获”等不严谨承诺改为可测量、可验证、可回退的工程标准。补全隐私合规、灾备退役、AI 辅助协议、依赖治理、判例库和极端架构启用条件。