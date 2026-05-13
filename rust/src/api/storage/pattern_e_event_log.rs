//! 场景关键词: 事件日志 / append-only / NDJSON / 审计 / replay / CRDT 源 → 选我。
//!
//! 模式 E: 追加式事件日志 (NDJSON 单文件)。
//!
//! 每行一个 JSON 对象, 字段定义在 [`StorageSampleEventDto`]。`append` 用 O_APPEND
//! 保证多写并发不会撕裂行 (POSIX 保证 <PIPE_BUF 字节的 write 原子)。
//!
//! 适用: 写多读少、按时间顺序消费、可全量 replay 的场景 (CRDT、审计)。
//! 不适用: 随机更新 / 删除 (用 KV); 跨多文件协调 (用 sqflite 事务)。
//!
//! `seq` 由调用方传入 (一般是单调自增计数器); 库**不维护**全局序号 — 这是
//! 沙箱标本, 真实业务可在外层用 sled 计数器 + 事务保证顺序写。
//!
//! 限制: `payload_json` 必须是合法 JSON 字符串 (单行); 校验只检查不含换行符,
//! 不做严格 JSON 解析 (留给消费方)。

use std::path::PathBuf;

use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::common::StorageError;

#[derive(Debug, Clone)]
pub struct StorageSampleEventDto {
    pub seq: u64,
    pub ts_ms: u64,
    pub kind: String,
    pub aggregate_id: String,
    /// 原样存储, 由消费方解析。要求单行 (不含 '\n')。
    pub payload_json: String,
}

const MAX_LINE_BYTES: usize = 256 * 1024; // 单条事件硬上限 256KB

/// 追加一条事件。文件不存在则创建。
pub async fn storage_sample_event_log_append(
    path: String,
    event: StorageSampleEventDto,
) -> Result<(), StorageError> {
    validate_event(&event)?;
    let target = PathBuf::from(&path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }
    let line = format!(
        "{{\"seq\":{},\"ts_ms\":{},\"kind\":{},\"aggregate_id\":{},\"payload\":{}}}\n",
        event.seq,
        event.ts_ms,
        json_string(&event.kind),
        json_string(&event.aggregate_id),
        event.payload_json
    );
    if line.len() > MAX_LINE_BYTES {
        return Err(StorageError::quota_exceeded(format!(
            "event line {} > {MAX_LINE_BYTES}",
            line.len()
        )));
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)
        .await?;
    f.write_all(line.as_bytes()).await?;
    f.sync_all().await?;
    Ok(())
}

/// 从 `from_seq` 起读所有事件 (inclusive)。文件不存在视为空。
/// 注意: 返回前会全部加载到内存; 大日志请用 `replay_from_with_limit`。
pub async fn storage_sample_event_log_replay_from(
    path: String,
    from_seq: u64,
) -> Result<Vec<StorageSampleEventDto>, StorageError> {
    storage_sample_event_log_replay_from_with_limit(path, from_seq, 0).await
}

/// 带上限版本。`limit=0` 表示不限。
pub async fn storage_sample_event_log_replay_from_with_limit(
    path: String,
    from_seq: u64,
    limit: u64,
) -> Result<Vec<StorageSampleEventDto>, StorageError> {
    let target = PathBuf::from(&path);
    let file = match fs::File::open(&target).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut out = Vec::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim_end_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        let ev = parse_event(trimmed)
            .ok_or_else(|| StorageError::corrupted(format!("malformed line: {trimmed}")))?;
        if ev.seq < from_seq {
            continue;
        }
        out.push(ev);
        if limit > 0 && out.len() as u64 >= limit {
            break;
        }
    }
    Ok(out)
}

/// 截断日志, 保留 `seq <= keep_until` 的事件。实现为 read → filter → atomic write。
pub async fn storage_sample_event_log_truncate_to(
    path: String,
    keep_until: u64,
) -> Result<u64, StorageError> {
    let target = PathBuf::from(&path);
    let all = storage_sample_event_log_replay_from(path.clone(), 0).await?;
    let before = all.len();
    let kept: Vec<_> = all.into_iter().filter(|e| e.seq <= keep_until).collect();
    let removed = (before - kept.len()) as u64;
    let mut body = String::new();
    for e in kept {
        body.push_str(&format!(
            "{{\"seq\":{},\"ts_ms\":{},\"kind\":{},\"aggregate_id\":{},\"payload\":{}}}\n",
            e.seq,
            e.ts_ms,
            json_string(&e.kind),
            json_string(&e.aggregate_id),
            e.payload_json
        ));
    }
    let tmp = {
        let mut t = target.clone();
        t.set_extension("tmp");
        t
    };
    // 写入 tmp 后 fsync, 再 rename — 与模式 B 同步语义, 保证 truncate 不会留半文件。
    let mut f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .await?;
    f.write_all(body.as_bytes()).await?;
    f.sync_all().await?;
    drop(f);
    fs::rename(&tmp, &target).await?;
    Ok(removed)
}

fn validate_event(e: &StorageSampleEventDto) -> Result<(), StorageError> {
    if e.kind.is_empty() {
        return Err(StorageError::invalid_argument("kind must not be empty"));
    }
    if e.aggregate_id.is_empty() {
        return Err(StorageError::invalid_argument(
            "aggregate_id must not be empty",
        ));
    }
    if e.payload_json.contains('\n') {
        return Err(StorageError::invalid_argument(
            "payload_json must be single-line",
        ));
    }
    Ok(())
}

/// 极简 JSON 字符串转义 (足够 sample 用; 真实系统用 serde_json)。
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 极简解析: 提取 seq / ts_ms / kind / aggregate_id / payload。
/// 严格依赖 [`storage_sample_event_log_append`] 的写出格式。
fn parse_event(line: &str) -> Option<StorageSampleEventDto> {
    let s = line.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return None;
    }
    let body = &s[1..s.len() - 1];
    let seq = extract_number(body, "\"seq\":")?;
    let ts_ms = extract_number(body, "\"ts_ms\":")?;
    let kind = extract_string(body, "\"kind\":")?;
    let aggregate_id = extract_string(body, "\"aggregate_id\":")?;
    let payload_json = extract_raw_after(body, "\"payload\":")?;
    Some(StorageSampleEventDto {
        seq,
        ts_ms,
        kind,
        aggregate_id,
        payload_json,
    })
}

fn extract_number(body: &str, key: &str) -> Option<u64> {
    let i = body.find(key)?;
    let after = &body[i + key.len()..];
    let end = after.find(|c: char| c == ',' || c == '}').unwrap_or(after.len());
    after[..end].trim().parse().ok()
}

fn extract_string(body: &str, key: &str) -> Option<String> {
    let i = body.find(key)?;
    let after = &body[i + key.len()..];
    let after = after.trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

fn extract_raw_after(body: &str, key: &str) -> Option<String> {
    let i = body.find(key)?;
    let raw = &body[i + key.len()..];
    Some(raw.trim().to_string())
}
