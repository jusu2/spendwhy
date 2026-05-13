# ADR 0004: 持久化层正规化与迁移基础设施

## 状态

**Accepted — 与本 ADR 一同提交的 sqflite v2 schema + v1→v2 迁移即为首版实施**。
见 [lib/data/database.dart](../../lib/data/database.dart)。
集成测试：[test/data/repository_contract_test.dart](../../test/data/repository_contract_test.dart)。

## 背景

当前持久化模型（[lib/models/fragment.dart](../../lib/models/fragment.dart) `toMap`/`fromMap`）严重反范式：

| 列 | 当前格式 | 问题 |
|---|---|---|
| `fragment.tags` | `'relationships,family,work'` | 违反 1NF；不可索引；含 `,` 即崩 |
| `fragment.image_paths` | `'p1\|p2\|p3'` | 同上；不可外键 |
| `recovery.related_ids` | `'frag-a,frag-b'` | 多对多关系塞进字符串；无 FK 约束，可指向已删除 fragment |

同时缺：

- `updated_at` / `deleted_at` / `revision`（无审计、无软删、无乐观锁）
- 数据库迁移机制（schema 改动 = 用户数据丢失）
- `STRICT` 表 / `CHECK` 约束
- 外键级联策略
- 索引（list_active fragment 全表扫）

## 决策

### 表结构（target schema v2）

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE fragment (
  id              TEXT    PRIMARY KEY,
  created_at_ms   INTEGER NOT NULL,
  updated_at_ms   INTEGER NOT NULL,
  deleted_at_ms   INTEGER,
  revision        INTEGER NOT NULL DEFAULT 1,
  intensity       INTEGER NOT NULL CHECK (intensity BETWEEN 1 AND 5),
  fade_period_days INTEGER NOT NULL CHECK (fade_period_days > 0),
  stage           TEXT    NOT NULL CHECK (stage IN ('outburst','recovery','relapse')),
  visibility      TEXT    NOT NULL CHECK (visibility IN ('private','anonymous')),
  -- 与 ADR-0002 协同：明文列在迁移后期被 cipher 列替换
  content_cipher  BLOB,
  content_nonce   BLOB,
  content_key_id  INTEGER
) STRICT;

CREATE INDEX idx_fragment_active
  ON fragment(deleted_at_ms, created_at_ms DESC);

CREATE TABLE fragment_tag (
  fragment_id TEXT NOT NULL REFERENCES fragment(id) ON DELETE CASCADE,
  tag         TEXT NOT NULL,
  PRIMARY KEY (fragment_id, tag)
) STRICT;

CREATE INDEX idx_fragment_tag_tag ON fragment_tag(tag);

CREATE TABLE fragment_image (
  fragment_id TEXT    NOT NULL REFERENCES fragment(id) ON DELETE CASCADE,
  ord         INTEGER NOT NULL,
  path        TEXT    NOT NULL,
  PRIMARY KEY (fragment_id, ord)
) STRICT;

CREATE TABLE recovery (
  id                  TEXT PRIMARY KEY,
  created_at_ms       INTEGER NOT NULL,
  updated_at_ms       INTEGER NOT NULL,
  deleted_at_ms       INTEGER,
  revision            INTEGER NOT NULL DEFAULT 1,
  intensity           INTEGER NOT NULL CHECK (intensity BETWEEN 1 AND 5),
  description_cipher  BLOB,
  description_nonce   BLOB,
  description_key_id  INTEGER
) STRICT;

CREATE INDEX idx_recovery_active
  ON recovery(deleted_at_ms, created_at_ms DESC);

CREATE TABLE recovery_fragment_link (
  recovery_id TEXT NOT NULL REFERENCES recovery(id) ON DELETE CASCADE,
  fragment_id TEXT NOT NULL REFERENCES fragment(id) ON DELETE CASCADE,
  PRIMARY KEY (recovery_id, fragment_id)
) STRICT;

CREATE TABLE crypto_meta (
  key_id      INTEGER PRIMARY KEY,
  created_at_ms INTEGER NOT NULL,
  retired_at_ms INTEGER
) STRICT;

CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at_ms INTEGER NOT NULL
) STRICT;
```

### 迁移机制

```rust
// rust/src/persistence/migration.rs
pub struct Migration { pub version: u32, pub up: fn(&Connection) -> AppResult<()> }
static MIGRATIONS: &[Migration] = &[
    Migration { version: 1, up: m1_initial },
    Migration { version: 2, up: m2_normalize_and_encrypt },
];

pub fn migrate(conn: &Connection) -> AppResult<()> {
    let current = current_version(conn)?;
    for m in MIGRATIONS.iter().filter(|m| m.version > current) {
        conn.transaction(|tx| { (m.up)(tx)?; record(tx, m.version) })?;
    }
}
```

迁移路径 v1 → v2：

1. 新建 v2 所有表
2. INSERT INTO new_fragment SELECT ... FROM old_fragment（拆 CSV tags / image_paths 到关联表）
3. INSERT INTO new_recovery SELECT ... FROM old_recovery（拆 related_ids 到链接表）
4. 加密 content / description（调 Vault.seal）
5. RENAME old → backup_v1，new → 主表
6. 备份表保留到下次启动验证后再 drop（用户保险）

### 仓储接口

```rust
pub trait FragmentRepository {
    fn find(&self, id: &FragmentId) -> AppResult<Option<Fragment>>;
    fn save(&self, fragment: &Fragment, expected_revision: u32) -> AppResult<u32>;
    fn soft_delete(&self, id: &FragmentId) -> AppResult<()>;
    fn list_active(&self, pagination: Page) -> AppResult<Vec<Fragment>>;
    fn list_by_tag(&self, tag: &Tag, pagination: Page) -> AppResult<Vec<Fragment>>;
}
```

`save` 校验 `revision` —— 不一致返回 `AppError::Conflict`，UI 提示"已被另一处修改"。

### Dart 退出策略

Dart 的 `Fragment.toMap/fromMap` 删除。所有持久化都走 Rust：

```dart
// 新
await RustBackend.createFragment(...);
final list = await RustBackend.listActiveFragments(page: 0);
```

ADR-0001 的退出条件 ✅ 由本 ADR 兑现。

## 约束 / 风险

1. **迁移不可逆**：必须备份 v1 数据到 `*_v1_backup` 表，至少跨两个版本不 drop。
2. **多对多 link 表迁移**：旧 `related_ids` CSV 中可能有指向已删除 fragment 的 stale id —— 迁移时丢弃这些条目并打日志。
3. **性能基线**：现网量级（< 1000 fragments）下迁移 < 1s。
4. **依赖 ADR-0002**：cipher 列需要 Vault；合并执行可减少一次大版本。

## 验收

- 单元：每个 migration 独立测试，输入旧 schema 样本数据 → 输出 v2 schema 数据相等且关系正确。
- 端到端：复制真实用户库一份，迁移后所有 fragment / recovery 可读且字段一致。
- Provider 测试：FragmentsProvider 不再依赖 Dart sqflite 直访。
