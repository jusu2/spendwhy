//! 场景关键词: 请求元数据 / request id / deadline / idempotency / traceparent / locale → 选我。
//!
//! 模式 V: 请求横切元数据 (Request Metadata)。
//!
//! 与"数据形状"正交的横切关注。其它任何模式 (A–U) 都可以**可选地**接受
//! 一个 `TransportRequestMeta` 作为第一参数, 把"我是谁、要多久内完成、
//! 出问题怎么追、是否幂等"这些通用语义统一处理, 不污染业务签名。
//!
//! # 数据形状三分法
//!
//! 本库不追求"万能 DTO"。Flutter → Rust 的数据按形状分三类:
//!
//! - **形状已知且稳定** → 各 pattern 自己定义强类型 DTO (绝大多数);
//! - **形状未知/动态** → 模式 R (`pattern_r_json`) 走 JSON 字符串透传;
//! - **数据大/二进制** → 模式 G (`Vec<u8>`) 或模式 S (文件路径 + sha256)。
//!
//! 横切元数据 (本模式) 不属于上面任何一类: 它是"请求上下文"而非"业务负载"。
//!
//! # 字段语义
//!
//! - `request_id`: 调用方生成的不透明 ID (推荐 UUID v4)。1..=128 ASCII。
//! - `idempotency_key`: 接收方 (如模式 L) 据此去重。Some 即开启幂等语义。
//! - `budget_ms`: 剩余预算 (ms)。**从 Dart 调用瞬间起算**, 超时立即返回
//!   `TransportError::timeout`。Dart 端 retry 时应重算后再传。
//! - `trace_parent`: W3C `traceparent` 头格式 `00-{32hex}-{16hex}-{2hex}`,
//!   分布式追踪用; 不解析, 仅转发。
//! - `locale`: BCP-47 (e.g. `zh-CN`); 影响 Rust 内部本地化决策。
//! - `attempt`: 1 = 首次; ≥2 = 重试。日志里据此区分。
//! - `source`: 调用方标识 (`ui.fragment_list`, `background.sync` 等); 观测用。

use std::time::Instant;

use flutter_rust_bridge::frb;

use super::common::TransportError;

/// 请求横切元数据。其他 pattern 可作可选第一参数接受。
#[derive(Debug, Clone)]
pub struct TransportRequestMeta {
    pub request_id: String,
    pub idempotency_key: Option<String>,
    pub budget_ms: Option<u64>,
    pub trace_parent: Option<String>,
    pub locale: Option<String>,
    pub attempt: u32,
    pub source: Option<String>,
}

/// 字段长度上限 (防 DoS / 防垃圾日志)。
pub const REQUEST_ID_MAX_LEN: usize = 128;
pub const IDEMPOTENCY_KEY_MAX_LEN: usize = 128;
pub const SOURCE_MAX_LEN: usize = 64;
pub const LOCALE_MAX_LEN: usize = 35; // BCP-47 极端长度
pub const TRACE_PARENT_LEN: usize = 55; // 固定: "00-" + 32 + "-" + 16 + "-" + 2

impl TransportRequestMeta {
    /// 校验所有字段; 任一违规返回 `TransportError::invalid_argument`。
    ///
    /// 校验规则:
    /// - `request_id` 非空、≤128、仅 ASCII 可见字符 (33..=126)。
    /// - `idempotency_key` 若 Some: 同上规则。
    /// - `budget_ms` 若 Some: > 0 (0 表示已到期 → 调用方应不要发起此请求)。
    /// - `trace_parent` 若 Some: 严格匹配 W3C 格式。
    /// - `locale` 若 Some: ≤35, ASCII 字母 / 数字 / `-`。
    /// - `attempt` ≥ 1。
    /// - `source` 若 Some: ≤64, ASCII 可见。
    #[frb(ignore)]
    pub fn validate(&self) -> Result<(), TransportError> {
        validate_id_like("request_id", &self.request_id, REQUEST_ID_MAX_LEN)?;
        if let Some(k) = &self.idempotency_key {
            validate_id_like("idempotency_key", k, IDEMPOTENCY_KEY_MAX_LEN)?;
        }
        if let Some(b) = self.budget_ms {
            if b == 0 {
                return Err(TransportError::invalid_argument(
                    "budget_ms must be > 0 (request already expired before send)",
                ));
            }
        }
        if let Some(tp) = &self.trace_parent {
            validate_trace_parent(tp)?;
        }
        if let Some(loc) = &self.locale {
            validate_locale(loc)?;
        }
        if self.attempt < 1 {
            return Err(TransportError::invalid_argument(
                "attempt must be >= 1 (first call = 1)",
            ));
        }
        if let Some(s) = &self.source {
            validate_id_like("source", s, SOURCE_MAX_LEN)?;
        }
        Ok(())
    }

    /// 把 budget_ms 转成绝对 deadline (在 Rust 内部计时, 不受 wall-clock 跳变影响)。
    #[frb(ignore)]
    pub fn deadline(&self) -> Option<Instant> {
        self.budget_ms
            .map(|b| Instant::now() + std::time::Duration::from_millis(b))
    }

    /// 在长任务中调用以检查是否已超过 deadline; 是则返回 `timeout`。
    /// `started_at` 应在任务起点用 `Instant::now()` 捕获。
    #[frb(ignore)]
    pub fn check_deadline(&self, started_at: Instant) -> Result<(), TransportError> {
        if let Some(budget) = self.budget_ms {
            let elapsed = started_at.elapsed().as_millis() as u64;
            if elapsed >= budget {
                return Err(TransportError::timeout(elapsed));
            }
        }
        Ok(())
    }

    /// 构造一个最小 meta (只填 request_id, 其它默认)。多用于内部/测试。
    #[frb(ignore)]
    pub fn minimal(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            idempotency_key: None,
            budget_ms: None,
            trace_parent: None,
            locale: None,
            attempt: 1,
            source: None,
        }
    }
}

fn validate_id_like(field: &str, v: &str, max: usize) -> Result<(), TransportError> {
    if v.is_empty() {
        return Err(TransportError::invalid_argument(format!(
            "{field} must not be empty"
        )));
    }
    if v.len() > max {
        return Err(TransportError::invalid_argument(format!(
            "{field} too long ({} > {max})",
            v.len()
        )));
    }
    if !v.bytes().all(|b| (33..=126).contains(&b)) {
        return Err(TransportError::invalid_argument(format!(
            "{field} contains non-printable or non-ASCII chars"
        )));
    }
    Ok(())
}

fn validate_trace_parent(tp: &str) -> Result<(), TransportError> {
    if tp.len() != TRACE_PARENT_LEN {
        return Err(TransportError::invalid_argument(format!(
            "trace_parent must be exactly {TRACE_PARENT_LEN} chars (W3C traceparent)"
        )));
    }
    let bytes = tp.as_bytes();
    // 形如: "00-{32}-{16}-{2}"  hex lowercase
    let dashes = [2, 35, 52];
    for &i in &dashes {
        if bytes[i] != b'-' {
            return Err(TransportError::invalid_argument(
                "trace_parent malformed: expected dashes at positions 2/35/52",
            ));
        }
    }
    for (i, &b) in bytes.iter().enumerate() {
        if dashes.contains(&i) {
            continue;
        }
        if !b.is_ascii_hexdigit() || (b.is_ascii_uppercase()) {
            return Err(TransportError::invalid_argument(
                "trace_parent must be lowercase hex",
            ));
        }
    }
    // version "00" 是当前唯一定义版本
    if &tp[0..2] != "00" {
        return Err(TransportError::invalid_argument(
            "trace_parent version must be '00'",
        ));
    }
    // trace_id / span_id 不能全 0
    if &tp[3..35] == "00000000000000000000000000000000" {
        return Err(TransportError::invalid_argument(
            "trace_parent trace_id must not be all zeros",
        ));
    }
    if &tp[36..52] == "0000000000000000" {
        return Err(TransportError::invalid_argument(
            "trace_parent span_id must not be all zeros",
        ));
    }
    Ok(())
}

fn validate_locale(loc: &str) -> Result<(), TransportError> {
    if loc.is_empty() || loc.len() > LOCALE_MAX_LEN {
        return Err(TransportError::invalid_argument("locale length 1..=35"));
    }
    if !loc
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(TransportError::invalid_argument(
            "locale must be ASCII letters / digits / dashes",
        ));
    }
    if loc.starts_with('-') || loc.ends_with('-') || loc.contains("--") {
        return Err(TransportError::invalid_argument(
            "locale: dashes only between segments",
        ));
    }
    Ok(())
}

/// 调用回执: 演示 meta 的端到端语义 (校验 → 接收 → 回显)。
#[derive(Debug, Clone)]
pub struct TransportSampleMetaReceiptDto {
    pub request_id: String,
    pub attempt: u32,
    pub elapsed_ms: u64,
    pub budget_remaining_ms: Option<u64>,
    pub locale_applied: Option<String>,
    /// 服务端处理 input 的回显; 真实业务里这里是结果 DTO。
    pub echoed_payload: String,
}

/// 演示入口: 任意业务调用都可以照抄这个签名 (meta 作可选第一参数)。
///
/// 行为:
/// 1. 校验 meta。
/// 2. 如 `budget_ms` 已耗尽, 立刻 `timeout`。
/// 3. 模拟 `work_ms` 的工作; 期间每 10ms 检查 deadline。
/// 4. 返回回执, 含剩余预算。
pub async fn transport_sample_with_meta(
    meta: TransportRequestMeta,
    payload: String,
    work_ms: u64,
) -> Result<TransportSampleMetaReceiptDto, TransportError> {
    meta.validate()?;
    let started = Instant::now();
    meta.check_deadline(started)?;

    if payload.len() > 4096 {
        return Err(TransportError::invalid_argument(
            "payload too large for sample (max 4096)",
        ));
    }

    let mut remaining = work_ms;
    while remaining > 0 {
        let step = remaining.min(10);
        tokio::time::sleep(std::time::Duration::from_millis(step)).await;
        remaining -= step;
        meta.check_deadline(started)?;
    }

    let elapsed_ms = started.elapsed().as_millis() as u64;
    let budget_remaining_ms = meta.budget_ms.map(|b| b.saturating_sub(elapsed_ms));

    Ok(TransportSampleMetaReceiptDto {
        request_id: meta.request_id,
        attempt: meta.attempt,
        elapsed_ms,
        budget_remaining_ms,
        locale_applied: meta.locale,
        echoed_payload: payload,
    })
}

/// 独立校验入口: Dart 侧 unit-test 用。生产业务**不需要**直接调它,
/// 因为 `transport_sample_with_meta` 已内置 validate。
pub fn transport_sample_validate_meta(meta: TransportRequestMeta) -> Result<(), TransportError> {
    meta.validate()
}
