//! 场景关键词: 一次性业务用例 / 数据库 / 加密 / 网络 / 可能阻塞 → 选我。
//!
//! 模式 B: 异步 Future。
//!
//! 何时用:
//! - 需要做 IO (访问 DB、文件、HTTP) 或 CPU 密集 (压缩、加密)。
//! - 调用模型是「请求 → 单一应答」的一次性 RPC。
//!
//! 何时不用:
//! - 要持续推送结果 → [`super::pattern_c_stream`]。
//! - 用户可能想中途取消 → [`super::pattern_d_cancel`]。
//! - 一次返回 List<List<Map<...>>> 之类深嵌套结构 → 改成分页 (模式 K) 或扁平列式。

use std::time::Duration;

use super::common::TransportError;

/// 示例: 异步执行一段业务用例; 这里以 `tokio::time::sleep` 模拟 IO。
pub async fn transport_sample_compute(input: String) -> Result<String, TransportError> {
    if input.is_empty() {
        return Err(TransportError::invalid_argument("input is empty"));
    }
    // 真实业务在这里换成: db.fetch(...).await? / http.get(...).await? 等
    tokio::time::sleep(Duration::from_millis(5)).await;
    Ok(format!("processed:{input}"))
}

/// 演示带超时的用例式 API。生产建议把超时作显式参数, 而不是隐式默认值。
pub async fn transport_sample_with_timeout(
    input: String,
    timeout_ms: u64,
) -> Result<String, TransportError> {
    let fut = transport_sample_compute(input);
    match tokio::time::timeout(Duration::from_millis(timeout_ms), fut).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(TransportError::timeout(timeout_ms)),
    }
}
