//! 场景关键词: 大二进制 / 图像 / 向量 / 文件 / 几 MB+ → 选我。
//!
//! 模式 G: 大二进制传输 / 分块流。
//!
//! 两种子模式:
//! 1. **单次返回**: 直接返回 `Vec<u8>`; FRB 2.x 已自动以 `Uint8List` 投递 (零额外拷贝)。
//! 2. **分块流**: 文件 >10MB 时, 用 `StreamSink<Vec<u8>>` 按 chunk 推送, Dart 侧拼接或边写边落盘。
//!
//! 何时不用:
//! - 数据在文件系统里 → 改 [`super::pattern_s_file_handoff`] (传路径)。

use crate::frb_generated::StreamSink;

use super::common::TransportError;

/// 单次返回大缓冲: Dart 侧拿到 `Uint8List`。
pub fn transport_sample_big_bytes(size: usize) -> Vec<u8> {
    let mut v = vec![0u8; size];
    for (i, b) in v.iter_mut().enumerate() {
        *b = (i % 256) as u8;
    }
    v
}

/// 把一段 payload 切块推给 Dart。
pub async fn transport_sample_chunk_stream(
    sink: StreamSink<Vec<u8>>,
    total_size: usize,
    chunk_size: usize,
) -> Result<(), TransportError> {
    if chunk_size == 0 {
        return Err(TransportError::invalid_argument("chunk_size must be > 0"));
    }
    let mut sent = 0usize;
    while sent < total_size {
        let take = chunk_size.min(total_size - sent);
        let chunk: Vec<u8> = (0..take).map(|i| ((sent + i) % 256) as u8).collect();
        if sink.add(chunk).is_err() {
            return Ok(());
        }
        sent += take;
        tokio::task::yield_now().await;
    }
    Ok(())
}
