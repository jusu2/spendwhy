//! 场景关键词: 分页 / 增量列表 / keyset cursor / 列式批量 → 选我。
//!
//! 模式 K: Keyset 分页 + 扁平列式 DTO。
//!
//! 设计要点:
//! - 不要返回深嵌套 `List<Map<...>>`; 用列式 (parallel arrays) 减少序列化代价。
//! - 用 `next_cursor` (上次最后一项的不透明 key), 而不是 offset。
//! - 一页大小 32~128, 不要 1000。

use super::common::TransportError;

#[derive(Debug, Clone)]
pub struct TransportSamplePage {
    pub ids: Vec<String>,
    pub titles: Vec<String>,
    pub created_at_ms: Vec<i64>,
    pub next_cursor: Option<String>,
}

/// 演示分页: 实际项目把 cursor 解码为「上次最后一项的 (created_at, id)」二元组。
pub async fn transport_sample_list_page(
    cursor: Option<String>,
    limit: u32,
) -> Result<TransportSamplePage, TransportError> {
    if limit == 0 || limit > 256 {
        return Err(TransportError::invalid_argument("limit must be in 1..=256"));
    }
    let start: u64 = match cursor.as_deref() {
        None | Some("") => 0,
        Some(s) => s
            .parse()
            .map_err(|_| TransportError::invalid_argument("malformed cursor"))?,
    };
    let end = start.saturating_add(limit as u64);
    let ids = (start..end).map(|i| format!("id-{i}")).collect();
    let titles = (start..end).map(|i| format!("title #{i}")).collect();
    let created_at_ms = (start..end).map(|i| 1_700_000_000_000 + i as i64).collect();
    Ok(TransportSamplePage {
        ids,
        titles,
        created_at_ms,
        next_cursor: Some(end.to_string()),
    })
}
