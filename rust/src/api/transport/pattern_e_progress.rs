//! 场景关键词: 进度条 / 大任务 + 取消 / 分阶段 → 选我。
//!
//! 模式 E: Stream<Progress> + CancelHandle 组合。
//!
//! Dart 侧:
//! ```dart
//! final cancel = await newCancelHandle();
//! final progress = transportSampleProgressJob(steps: 100, cancel: cancel);
//! progress.listen((p) => setState(() => fraction = p.fraction));
//! ```
//!
//! 设计:
//! - **节流**: 即便业务 step 数 = 1e6, 进度事件最多 100 条 (`emit_every`); 避免 UI 饥饿。
//! - **完成标记**: 最后一次 emit 必为 `fraction=1.0, stage="done"`, Dart 端据此切换状态。
//! - **取消语义**: 取消时返回 `canceled`, sink 自然结束 (不会再 emit)。

use std::time::Duration;

use crate::frb_generated::StreamSink;

use super::common::{CancelHandle, ProgressDto, TransportError};

pub async fn transport_sample_progress_job(
    sink: StreamSink<ProgressDto>,
    steps: u32,
    step_duration_ms: u64,
    cancel: &CancelHandle,
) -> Result<(), TransportError> {
    if steps == 0 || steps > 1_000_000 {
        return Err(TransportError::invalid_argument(
            "steps must be in 1..=1_000_000",
        ));
    }
    let token = cancel.token();
    let total = steps;
    // 高 step 数下节流: 最多发 100 条进度事件 (含完成)。
    let emit_every = (total / 100).max(1);
    let step_d = Duration::from_millis(step_duration_ms);

    for i in 1..=total {
        token.check()?;
        if step_d.is_zero() {
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(step_d).await;
        }
        let is_last = i == total;
        if i % emit_every == 0 || is_last {
            let p = ProgressDto {
                fraction: i as f64 / total as f64,
                stage: if is_last { "done".into() } else { "running".into() },
                message: None,
            };
            if sink.add(p).is_err() {
                return Ok(()); // 订阅方已离开
            }
        }
    }
    Ok(())
}
