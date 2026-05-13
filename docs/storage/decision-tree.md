# Storage 决策树

按问题顺序往下走, 锁定模式编号。

## Step 1: 数据要不要在进程重启后还在?

- 不要 (临时缓存、加速热路径) → **模式 A** (`pattern_a_memory`)
- 要 → Step 2

## Step 2: 一次最大多大?

- 小值 (≤4KB, KV 形状) → Step 3
- 中等结构化 (一个 JSON / 配置 / 单条记录) → **模式 B** (原子写) 或 **模式 G** (设置 KV)
- 大对象 (>64KB blob / 文件) → **模式 C** (`pattern_c_blob`, 内容寻址)
- 流式追加 (日志、事件) → **模式 E** (`pattern_e_event_log`)

## Step 3: KV 还是关系?

- 简单 key→value, 需要前缀 / 范围扫描 → **模式 F** (`pattern_f_ordered_kv`, sled)
- 纯 settings / preferences (小、明文、常读) → **模式 G** (`pattern_g_settings`)
- 多表 / JOIN / 事务 → Dart 侧用 sqflite + **模式 L** (`sql.dart`) + **模式 M** (`migration.dart`)
- 想要 TTL + LRU 持久缓存 → **模式 H** (`pattern_h_persistent_cache`)

## Step 4: 安全性要求?

- 普通业务数据 → 默认 (不加密, 走 OS 文件权限)
- 凭据 / Token / 主密钥 / 生物锁后才解的值 → **模式 K** (`secure_storage.dart`)
- 自由文本字段需要静态加密 → **模式 I** (`pattern_i_encryption`, AES-256-GCM)
  + 主密钥从模式 K 取

## Step 5: 横切需求?

- **撤销 / undo / 回滚点**: **模式 D** (`pattern_d_snapshot`) 或 **模式 Q** (`soft_delete.dart`)
- **备份 / 导出 / 跨设备迁移**: **模式 J** (`pattern_j_backup`)
- **离线写, 上线再同步**: **模式 N** (`outbox.dart`)
- **多用户 / workspace 隔离**: **模式 O** (`tenant.dart`)
- **大表 schema 演化 / 字段补算**: **模式 M** (`migration.dart`) + **模式 P** (`backfill.dart`)
- **L1 内存 + L2 持久双层缓存**: **模式 R** (`cache_combinator.dart`)

## 常见场景 → 模式映射

| 场景 | 模式组合 |
|---|---|
| 用户偏好 (主题、语言) | G |
| 登录 Token / Refresh Token | K |
| 缓存远端列表 30 分钟 | A 或 H |
| 大文件 (相片、PDF) 落本地 | C |
| 用户每次编辑都能 undo 5 步 | D (快照) 或 Q (软删) |
| 审计日志 / 行为追踪 | E |
| 接近 SQL 查询的本地数据 | sqflite + L + M |
| 加密用户笔记内容 | I (内容) + K (主密钥) |
| 备份整个应用数据到 USB | J |
| 离线时下单, 联网后同步 | N |
| SaaS 多账号切换 | O |
| 数据库加字段后补算历史值 | P |
| 删除"撤销" 30 天后才真删 | Q |
| HTTP 响应缓存 (内存+磁盘) | R = L1(A) + L2(H) |
| 历史范围查询 (`2024-01..2024-03`) | F (key=`yyyymm`, range scan) |

## 我应该选 Rust 侧还是 Dart 侧?

| 倾向 | 选 Rust (A..J) | 选 Dart (K..R) |
|---|---|---|
| 需要跨平台一致行为 | ✓ | (Dart 也行但慢些) |
| 已经在 Rust 持有数据 | ✓ | — |
| 需要 OS Keychain / Keystore | — | ✓ (模式 K) |
| 需要 sqflite 已有 schema | — | ✓ (模式 L/M/N/Q) |
| 性能敏感, 大批写 | ✓ (sled + 异步 IO) | — |
| 纯 UI 状态持久化 | — | ✓ |

当不确定时, 默认 Dart (距 UI 近, 调试快); 性能或安全有硬要求再下到 Rust。
