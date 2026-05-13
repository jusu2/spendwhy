# ADR 0005: 领域事件流与时间线视图

## 状态

Proposed — 计划在 ADR-0003/0004 落地后实施。

## 背景

SpendWhy 是一个"记录低谷 → 见证愈合"的产品。当前 UI 呈现：

- 首页：碎片列表（按 fade_level 排序的现在状态）
- 成长页：单一 growth_score 数字 + 平滑曲线

但产品的**核心叙事单元是"事件"**：

> 6 个月前我记录了一块关系类碎片。3 个月前一次和朋友的对话让我感到松动。上周我把它推进到了恢复期。

这条叙事链当前不存在 —— 数据库只有"现状"，没有"过程"。

## 决策

引入**轻量级 event log**（非 event sourcing），既保证 SSOT 仍是聚合根，又把过程显化。

### 事件类型

```rust
// rust/src/domain/event.rs
pub enum DomainEvent {
    FragmentCreated   { id: FragmentId, at: AppTime, intensity: Intensity, tags: Vec<Tag> },
    FragmentEdited    { id: FragmentId, at: AppTime, diff: FragmentDiff },
    FragmentStageAdvanced { id: FragmentId, at: AppTime, from: Stage, to: Stage, by_recovery_id: Option<RecoveryId> },
    FragmentDeleted   { id: FragmentId, at: AppTime },
    RecoveryRecorded  { id: RecoveryId, at: AppTime, intensity: Intensity, fragment_ids: Vec<FragmentId> },
    RecoveryDeleted   { id: RecoveryId, at: AppTime },
}
```

### 持久化

```sql
CREATE TABLE event_log (
  seq         INTEGER PRIMARY KEY AUTOINCREMENT,
  occurred_at_ms INTEGER NOT NULL,
  kind        TEXT NOT NULL,
  aggregate_id TEXT NOT NULL,
  payload     BLOB NOT NULL,    -- bincode 序列化的 DomainEvent
  payload_cipher BLOB,          -- 如果 payload 含敏感字段则走加密（同 ADR-0002）
  payload_nonce  BLOB,
  payload_key_id INTEGER
) STRICT;

CREATE INDEX idx_event_log_aggregate ON event_log(aggregate_id, seq);
CREATE INDEX idx_event_log_time ON event_log(occurred_at_ms DESC);
```

`seq` 单调，是 append-only 序列，**永远不更新不删除**（软删走 `*Deleted` 事件）。

### 写路径

```rust
// rust/src/application/recovery.rs
pub fn record_recovery(input) -> AppResult<(Recovery, Vec<DomainEvent>)> {
    let recovery = ...;
    let mut events = vec![DomainEvent::RecoveryRecorded { ... }];
    for f in advance_targets { events.push(DomainEvent::FragmentStageAdvanced { ... }); }
    Ok((recovery, events))
}

// Repository.save 内部把事件追加到 event_log（同事务）
```

事件与状态变更同事务原子提交。

### 读路径 / 时间线视图

```rust
#[flutter_rust_bridge::frb(sync)]
pub fn list_timeline(before: AppTimeDto, limit: u32) -> AppResult<Vec<TimelineEntryDto>>;
```

UI 直接消费 `TimelineEntryDto`，每条目自带 occurred_at、kind、本地化 label payload。  
"成长页"未来可以从一条数字进化为一条叙事时间线。

### 不上 Event Sourcing

不采用纯 event sourcing（即"实体由事件重建"）的原因：

- 复杂度过高，单人应用不需要 audit log 级别的回溯。
- 事件仅作为「附加视图」存在，删之不影响系统正确性。
- 未来若需 CRDT / E2EE 同步，再考虑事件作为同步单位。

## 约束 / 风险

1. **事件载荷膨胀**：若每条 FragmentEdited 都存 diff，长期增长可观；策略 — 90 天后压缩（合并连续 edit 为一个 `compacted_edit`）。
2. **加密事件载荷**：fragments_to_advance 列表本身无明文，但 diff/payload 中含 content snippet 时必须走 Vault。
3. **schema 演进**：bincode 不向后兼容；event payload 使用 `serde_json` + schema_version 字段，写一次读多次。

## 验收

- 每个 use case 单元测试包含「期望事件序列」断言（given/when/then 风格）。
- 端到端：record_recovery → 查询 timeline → 期望出现 `RecoveryRecorded` + N 个 `FragmentStageAdvanced`。
- 性能：1000 条 timeline 拉取 < 50ms（典型移动设备）。

## 不在本 ADR 范围

- 跨设备同步（事件作为同步单位是未来扩展点）。
- 事件的搜索 / 全文索引。
- 用户主动编辑事件（事件不可编辑，编辑实体本身会产生新事件）。
