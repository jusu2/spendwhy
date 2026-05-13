//! 场景关键词: 动态 schema / JSON / 第三方 API 包装 / schemaless → 选我。
//!
//! 模式 R: JSON 字符串透传。
//!
//! 何时用:
//! - 数据 schema 在 Rust 编译期未知 (第三方 API 响应、用户自定义模板)。
//! - 跨边界只需保留结构, Rust 不解码。
//!
//! 何时不用:
//! - schema 稳定 → 写真正的 DTO struct (类型安全 + 更省序列化)。
//!
//! 此模式不依赖 `serde_json`: Rust 视 payload 为不透明 `String`,
//! 让 Dart 端 `dart:convert` 解码。如确实要 Rust 端处理, 再加 serde。

use super::common::TransportError;

/// Echo: Dart 侧把 `jsonEncode(obj)` 传入, Rust 加 trace 字段后回传。
///
/// 演示"不解码也能加工"的边界处理。生产里可换成"加签名 / 加包裹元数据"。
pub async fn transport_sample_json_passthrough(payload_json: String) -> Result<String, TransportError> {
    let trimmed = payload_json.trim();
    if trimmed.is_empty() {
        return Err(TransportError::invalid_argument("payload_json is empty"));
    }
    // 极轻量的存在性检查; 真正 schema 校验留给 Dart 端或 serde 解码路径。
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return Err(TransportError::invalid_argument(
            "payload must be JSON object or array",
        ));
    }
    Ok(format!(
        r#"{{"trace":"rust","len":{},"inner":{}}}"#,
        trimmed.len(),
        trimmed
    ))
}
