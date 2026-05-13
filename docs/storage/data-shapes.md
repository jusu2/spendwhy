# 数据形状 (Data Shapes) 决策指南

> 一句话: **持久化先选引擎, 再选形状, 最后叠语义**。本库按 *引擎 / 持久度 /
> 形状* 三轴分流, 各轴尽量正交, 模式之间可组合。

---

## 三轴分流

### 轴 1: 引擎 (Where)

| 引擎 | 适用 | 模式 |
|---|---|---|
| 进程内存 | 临时缓存, 重启可丢 | A |
| 单文件 (atomic write) | 配置 / 单条状态 / 单个 JSON | B |
| 内容寻址目录 (blob) | 大对象, 去重 | C |
| 快照目录 | undo / 回滚点 | D |
| NDJSON 追加文件 | 事件日志 / 审计 | E |
| sled (ordered KV) | 前缀扫描 / 范围查询 / 二级缓存 | F, G, H |
| sqflite | 多表关系 / 事务 / JOIN | L, M, N, P, Q (Dart 侧) |
| OS Keychain / Keystore | 凭据 / 主密钥 | K |

### 轴 2: 持久度 (How long)

| 持久度 | 引擎候选 |
|---|---|
| 进程级 (重启丢) | A |
| 应用级 (卸载丢) | B, C, D, E, F, G, H, sqflite |
| 用户级 (跨重装可恢复) | K (Keychain 在 iOS 默认跨重装), J (备份导出) |

### 轴 3: 形状 (What)

| 形状 | 模式 |
|---|---|
| KV (key → bytes / string) | A, F, G, H |
| 单文件原文 | B |
| 大 blob (内容寻址) | C |
| 时间序列 / 追加流 | E |
| 关系表 | sqflite + L/M |
| 加密信封 (任意上层 + AES-GCM) | I |

---

## 决策树: 我现在要存的数据属于哪类?

### Step 1: 还需要在重启后吗?

```
否 → 模式 A
是 → Step 2
```

### Step 2: 数据大小?

```
≤ 4 KB key→value  → Step 3
4 KB ~ 64 KB 文档 → 模式 B (atomic write) 或 模式 F (sled, 如需扫描)
> 64 KB blob       → 模式 C (内容寻址)
追加流             → 模式 E
```

### Step 3: KV 访问模式?

```
点查 / 偶尔写        → 模式 G (settings)
前缀扫描 / 范围查询  → 模式 F (ordered KV)
带过期时间的缓存     → 模式 H (persistent cache)
```

### Step 4: 需要加密?

```
否 → 直接用上面选的引擎
是 → 模式 I 包一层 (encrypt → 把密文作为 value 存进 F/G/B/C 任意一个)
    主密钥从模式 K 取
```

### Step 5: 需要横切?

```
撤销 / 回滚      → 模式 D (快照) 或 模式 Q (软删)
备份 / 跨设备    → 模式 J
离线 → 同步      → 模式 N (outbox)
多用户隔离       → 模式 O (tenant)
schema 演化      → 模式 M + P
两层缓存 (L1+L2) → 模式 R 组合内存 + 持久
```

---

## 引擎选型: sled vs sqflite

| 维度 | sled (Rust) | sqflite (Dart) |
|---|---|---|
| 形状 | KV | 关系表 |
| 写吞吐 | 高 (LSM-like) | 中 |
| 范围扫描 | ✓ (key 字节序) | ✓ (SQL ORDER BY) |
| 事务 | 单 tree 内 | 跨表 |
| JOIN | ✗ | ✓ |
| 二进制大小 | ~500KB | 已在 (sqflite plugin) |
| 跨平台一致性 | 强 (Rust 实现) | 依赖平台 SQLite |

**默认**: 偏关系 / 多表 → sqflite; 偏 KV / 字节序 / 写多 → sled。

## 引擎选型: atomic file vs sled vs sqflite

- **单条全量替换** (e.g. 一份 JSON 配置, 每次都覆盖) → atomic file (模式 B), 没必要装 sled。
- **频繁更新 + 增量** → sled (模式 F/G/H), 避免 atomic file 的"每次重写全文件"开销。
- **强结构 + 查询** → sqflite, 写 SQL 比手工字节序拼 key 强。

---

## 反模式 (Don't)

1. **不要用 settings 模式 (G) 存 1MB 大值**: 它做了大小校验, 会返回 `quota_exceeded`;
   即使绕过校验, sled 也不擅长此 size。
2. **不要把 blob (C) 当 KV 用**: blob ID 是 sha256 派生, 不能任意命名; 想任意命名用 F。
3. **不要在 settings (G) 里建索引 / 排序 key**: 它是无序 KV, 要序就用 F。
4. **不要在 event log (E) 里删中间行**: 它是 append-only, 删除会触发 corrupted; 真要删用快照 + 重写。
5. **不要把主密钥写进 sled / sqflite**: 用模式 K (OS Keychain); 业务密钥才用信封 (模式 I)。
6. **不要在 backfill (P) 的 handle 里做非幂等操作** (e.g. 调用第三方 API): cursor 在
   每批后保存, 中途崩溃会重跑最后一批。
7. **不要靠 tombstone (Q) 永久保留**: 配 GC, 否则表无限膨胀, 查询变慢。
