//! panic 隔离守门员。
//!
//! 两个入口 —— [`guard_sync`] 和 [`guard_async`] —— 共享三条设计要点,
//! 用之前请先读懂:
//!
//! 1. **捕获即终结。** 被捕到的 panic 会转成 [`AppError::Internal`], 前缀
//!    是 [`PANIC_INTERNAL_TAG`] (`"panic: "`)。下游代码无需解析文案就能
//!    区分 "use case 主动抛 Internal" vs "运行时捕到了 panic"。
//!    这个前缀是公开契约的一部分。
//!
//! 2. **不重复捕获。** 一个被守门员包过的调用再被外层守门员包一次是安全
//!    且幂等的: 内层守门员已经把 panic 转成 `AppError`, 外层看到的就是
//!    普通 `Err`, 不会再触发上报。这样日志不重复。
//!
//! 3. **`AssertUnwindSafe` 是调用方的承诺。** 被守门员包住的闭包/future
//!    不能在 panic 时留下半成品状态 (持锁中写一半文件、刷一半缓冲等)。
//!    共享状态优先用 `parking_lot::Mutex` (无 poison) 或在 `Drop` 里显式回滚。

use std::panic::AssertUnwindSafe;

use archforge_kernel::AppError;
use futures::FutureExt;

use crate::reporter::{report_panic, PanicEvent};

/// 由捕获 panic 产生的每个 `AppError::Internal` 都带的固定前缀。
/// 跨版本稳定; **不要**依赖前缀之后的具体文案。
pub const PANIC_INTERNAL_TAG: &str = "panic: ";

/// 同步守门员。
///
/// 跑完 `f`。如果 panic, 提取 payload, 喂给已装好的 [`PanicReporter`],
/// 然后返回 `Err(AppError::Internal("panic: <message>"))`。
/// 业务自身的 `Err` 直通, 不动。
///
/// [`PanicReporter`]: crate::PanicReporter
///
/// # 示例
///
/// ```
/// use archforge_ffi::guard_sync;
/// use archforge_kernel::AppError;
///
/// let r: Result<u32, AppError> = guard_sync(|| Ok(42));
/// assert_eq!(r.unwrap(), 42);
///
/// let r: Result<u32, AppError> = guard_sync(|| panic!("oops"));
/// assert!(matches!(r, Err(AppError::Internal(_))));
/// ```
pub fn guard_sync<T, F>(f: F) -> Result<T, AppError>
where
    F: FnOnce() -> Result<T, AppError>,
{
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(ok) => ok,
        Err(payload) => Err(panic_to_app_error(payload, "guard_sync")),
    }
}

/// 异步守门员。
///
/// `await` 整个 `fut`。任何 `.await` 点 panic 都会被捕到、上报, 然后转成
/// `Err(AppError::Internal("panic: <message>"))`。业务自身的 `Err` 直通。
///
/// # 示例
///
/// ```
/// # use archforge_ffi::guard_async;
/// # use archforge_kernel::AppError;
/// # async fn run() {
/// let r: Result<&'static str, AppError> = guard_async(async { Ok("hi") }).await;
/// assert_eq!(r.unwrap(), "hi");
///
/// let r: Result<u8, AppError> = guard_async(async { panic!("nope") }).await;
/// assert!(matches!(r, Err(AppError::Internal(_))));
/// # }
/// # futures::executor::block_on(run());
/// ```
pub async fn guard_async<T, F>(fut: F) -> Result<T, AppError>
where
    F: std::future::Future<Output = Result<T, AppError>>,
{
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(ok) => ok,
        Err(payload) => Err(panic_to_app_error(payload, "guard_async")),
    }
}

fn panic_to_app_error(payload: Box<dyn std::any::Any + Send>, site: &'static str) -> AppError {
    let message = extract_panic_message(&*payload);
    report_panic(PanicEvent {
        site,
        message: &message,
    });
    AppError::Internal(format!("{PANIC_INTERNAL_TAG}{message}"))
}

/// 尽力从 panic payload 中提取可读字符串。常见两种 payload (`&'static str`
/// 和 `String`) 都能直接拿到; 其它类型退化到下面的占位文本, 至少让日志
/// 有一个确定性的输出。
fn extract_panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_ok_passes_through() {
        let r: Result<u32, AppError> = guard_sync(|| Ok(7));
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn business_err_passes_through_unchanged() {
        let r: Result<u32, AppError> = guard_sync(|| Err(AppError::NotFound("x".into())));
        assert!(matches!(r, Err(AppError::NotFound(s)) if s == "x"));
    }

    #[test]
    fn static_str_panic_is_caught_with_tag() {
        let r: Result<u32, AppError> = guard_sync(|| panic!("static-str"));
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("static-str"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn string_panic_is_caught_with_tag() {
        let r: Result<u32, AppError> = guard_sync(|| panic!("dynamic-{}", String::from("string")));
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("dynamic-string"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn non_string_panic_payload_degrades_gracefully() {
        let r: Result<u32, AppError> = guard_sync(|| std::panic::panic_any(42_u32));
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("<non-string panic payload>"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn nested_guards_do_not_double_wrap() {
        // 内层守门员先把 panic 转掉; 外层只看到 Internal 这个 Err, 不会再加 tag。
        let r: Result<u32, AppError> = guard_sync(|| guard_sync(|| panic!("once")));
        match r {
            Err(AppError::Internal(msg)) => {
                let occurrences = msg.matches(PANIC_INTERNAL_TAG).count();
                assert_eq!(occurrences, 1, "tag must not be doubled: {msg}");
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn async_business_ok_passes_through() {
        let r: Result<&'static str, AppError> = guard_async(async { Ok("hi") }).await;
        assert_eq!(r.unwrap(), "hi");
    }

    #[tokio::test]
    async fn async_panic_is_caught_with_tag() {
        let r: Result<u32, AppError> = guard_async(async { panic!("async-boom") }).await;
        match r {
            Err(AppError::Internal(msg)) => {
                assert!(msg.starts_with(PANIC_INTERNAL_TAG));
                assert!(msg.contains("async-boom"));
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn async_panic_across_await_is_caught() {
        // 验证守门员能跨越 yield 点继续工作。
        let r: Result<u32, AppError> = guard_async(async {
            tokio::task::yield_now().await;
            panic!("after-yield");
        })
        .await;
        assert!(matches!(r, Err(AppError::Internal(_))));
    }
}
