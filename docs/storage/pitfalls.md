# Storage 通用陷阱清单

## 跨边界共性

1. **不要把 Rust `Result` 错误原文透传到 UI**。统一过 `StorageError` (与 `transport` 同规)。
2. **不要返回深嵌套结构**。本库 DTO 都是扁平的 (避免 FRB freezed 依赖)。
3. **路径分隔符**: 跨平台一律用 `/` (Rust 用 `PathBuf` 自动适配), Dart 侧用 `path` 包拼接。
4. **时间戳**: 统一 i64 / u64 毫秒 (UTC), UI 层再格式化。

## 模式 A (内存缓存) 陷阱

- 没有持久; 重启就丢 — 不要把"唯一副本"放这。
- FIFO eviction (按 seq); 不是真 LRU, 不更新 access time。要更精确语义用模式 H。
- TTL 是惰性检查 (get 时才算); 不会自动后台清。

## 模式 B (原子文件写) 陷阱

- **不要绕过 atomic_write 直接 `fs::write`**: 断电会留半文件, 后续读取报 corrupted。
- **fsync 默认开启**: 慢但安全。批量写场景显式传 `fsync=false`, 但要自己接受丢失风险。
- 临时文件 `.{name}.tmp`: 若进程崩溃, 下次启动可能残留 — 安全 (本体未污染), 但偶尔要 GC。

## 模式 C (Blob) 陷阱

- ID = sha256(content) 前 32 hex; 不能反向解析"这是什么"。要附加元数据用旁路 manifest。
- 不能改; 改 = 新 ID + 旧 ID 留着。删要业务侧追踪引用计数。
- 2 级目录 bucketing (`xx/xxxxx...`); 不要直接平铺 100k 文件到单目录, FS 性能会崩。

## 模式 D (快照) 陷阱

- 快照不会自动 prune; 要主动调 `prune(keep_last_n)` 或定期 cron。
- 大文件每次快照都全拷贝; 不做差分。差分需求另开 pattern (本库未实现)。
- label 是文件名一部分, 校验只允许 `[a-z0-9_-]`; 不要塞 emoji / 中文。

## 模式 E (Event log) 陷阱

- NDJSON 单行 ≤ 256KB; 大 payload 拆 blob (模式 C) + 在日志只存 blob_id。
- truncate_to(seq) 是 read-rewrite-replace, 不是 O(1); 大日志慎用, 改为分段 (本库未实现)。
- replay 读到 corrupted 行 → 立即返回 corrupted, 不"跳过坏行"。坏行的修复需人工。

## 模式 F (sled ordered KV) 陷阱

- sled 0.34 是稳定老版; 0.35 / 1.0 一直 alpha — 不要随手升级。
- key 字节序 = 字典序; 数字 key 要 zero-pad 或用 big-endian 编码, 否则 "10" < "9"。
- sled tree 是单进程独占; 不要在两个 Rust process 同时打开同一 db, 会损坏。
- 异步 IO 都包 `spawn_blocking`; 不要直接 await sled 调用 (它是 sync API)。

## 模式 G (Settings) 陷阱

- key 限 `[A-Za-z0-9._-]`, value ≤4KB, 总 keys ≤4096 — 超限报 `quota_exceeded` / `invalid_argument`。
- 不要往这塞 list / map; 强结构请用 sqflite 或 atomic JSON file (模式 B)。

## 模式 H (持久缓存) 陷阱

- header 16 字节 (8B expires + 8B reserved); 读到短于 16 字节的 value → corrupted。
- 不要手动用 sled 同 tree 写裸 value, 会破 header 约定; 全部走本模式 API。
- GC 是显式调用; 不会自动跑。要在启动期或定时器里调 `gc_expired`。

## 模式 I (加密) 陷阱

- nonce 必须是真随机 (本库用 `OsRng`); 不要复用 nonce — 同 key + 同 nonce + 不同 plaintext = key 泄漏。
- AAD (Additional Authenticated Data) 不加密但鉴权; 用它绑定 record_id / version, 防换包攻击。
- 主密钥不要写代码 / 配置 / sled 里; 从模式 K (`secure_storage`) 取, 首次启动随机生成。
- 解密失败一律 `corrupted`; 不要给攻击者区分"key 错"和"data 改"的信息。

## 模式 J (备份) 陷阱

- 自实现归档格式 (manifest + base64 NDJSON), 非标准 tar; 跨工具不可读。
- 单包默认 ≤100MB; 大于此值切多包 (调用方实现)。
- 备份**不加密**; 含敏感数据时调用方先用模式 I 加密再 backup, 或备份后再加密整包。
- 恢复时先校验 manifest sha256; 不匹配 → `corrupted`, 拒绝任何写入。

## 模式 K (SecureStorage) 陷阱

- iOS `accessibility=first_unlock`: 设备首次解锁后才能读 — 后台任务在重启未解锁状态下读会失败。
- Android: 默认走 EncryptedSharedPreferences (新版自动); 不再显式传 deprecated 参数。
- listKeys 在 iOS 是 `readAll` 全表扫, 不要在热路径调; 主要给诊断 / 迁移用。

## 模式 L (SQL 事务) 陷阱

- 嵌套 = savepoint, 不是真嵌套事务; 外层 rollback 会废掉内层 release。
- `timeout` 仅在事务开始**之前**起作用; sqflite 没暴露中断 API。

## 模式 M (Migration) 陷阱

- 每个 migration 必须**幂等可重跑**: 即使 `runUp` 中途断电, 下次启动重跑同一版本不能崩。
- `down` 不是所有 migration 都需要; 但生产数据库要回滚就必须先写。
- 不要在 migration 里跑业务函数; 用 backfill (模式 P) 分离, migration 只改 schema。

## 模式 N (Outbox) 陷阱

- `idempotency_key` 必须**业务唯一**; 用 `${op}-${entityId}-${version}` 而非 random uuid (否则重发会变两条)。
- 死信不会自动清; `deadLetterCount()` 仅观测, 要业务侧人工 / 定时清理。
- worker 不在本库范围 (本库只提供 enqueue/dequeue/ack/nack); 自行用 Timer / isolate 跑。

## 模式 O (Tenant) 陷阱

- tenantId 校验严: `^[a-z0-9_-]{1,64}$`; 不允许大写 / 中文 / 空格 — 防 path-traversal / 注入。
- 切租户不是清缓存; 上层缓存 (模式 A/H) 要自己按租户分 namespace 或切换时 invalidate。
- 这层不做强隔离; 真要强隔离 (审计 / 合规) 用独立 DB 文件 + 独立主密钥。

## 模式 P (Backfill) 陷阱

- `handle` 必须**幂等**: cursor 在批后保存, 崩溃会重跑最后一批 — 重跑同 item 不能产生副作用差异。
- 大批 (batchSize 100+) + 复杂事务 = 长事务锁, 阻塞前台 → 把 batchSize 控小或夜间跑。
- reset 会丢 cursor 重头跑, 慎用; 通常只在 schema 改后再重做时用。

## 模式 Q (Soft delete) 陷阱

- 业务查询必须显式带 `activeWhereClause`; 否则 tombstone 也会被查出。
- 唯一索引会 conflict (deleted_at 的行还在表里); 业务表的 unique 约束要在条件里加
  `WHERE deleted_at IS NULL` (SQLite 支持 partial index)。
- GC 阈值 = 保留期; 太短失去 undo 意义, 太长表膨胀。常用 7~30 天。

## 模式 R (Cache combinator) 陷阱

- L2 故障要降级到 source, 不能让 cache 故障击穿业务; 本库 `_safe` 包了 L1/L2 调用。
- write-back: `put()` 完成只意味着 L1 写好; L2 还在异步刷, 关闭前要 `waitAllFlush()`。
- 不适合写多读少 (write-back 的异步刷会成瓶颈) — 那种用 write-through。

## Rust 异步惯例

- 所有 sled / fs / aes-gcm 阻塞操作必须包 `tokio::task::spawn_blocking`; 否则会拖垮 tokio runtime。
- 路径都从参数注入, 不要在 Rust 内部调 `path_provider` / 环境变量 — 测试与生产无法隔离。
- `OnceLock<Mutex<HashMap<path, Db>>>` 池化 sled connection; sled::Db 本身是 Arc-cheap-clone 的。
