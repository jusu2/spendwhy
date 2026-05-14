//! 全局可装的 panic 上报器。
//!
//! 守门员捕到一个 panic 时, 运行时要做**两**件事:
//!
//! 1. 告诉宿主 (Dart / C / 哪里都行) "这次调用挂了" —— 由
//!    [`crate::guard_sync`] / [`crate::guard_async`] 返回
//!    `AppError::Internal` 来完成。
//! 2. 告诉运维 (tracing / Sentry / 结构化日志) "刚才有 panic 跨过了边界, 文案
//!    在这里" —— 由本模块完成。
//!
//! 上报器在**进程启动时装一次**, 之后再装会被忽略。这样契约简单可审计;
//! 生产环境里不应该有任何代码在运行中翻转上报器。测试可以通过自家 mock
//! 的内部可变性绕开这点。
//!
//! 如果没装上报器, panic 还是会被捕获、回给调用方 —— 只是不写日志。失败
//! 静默 (fail closed) 比失败开放 (fail open) 安全, 后者意味着缺一个 logger
//! 就能阻塞 FFI 返回。

use std::sync::OnceLock;

/// 投递给 [`PanicReporter`] 的一次 panic 事件。借引用字符串避免在错误热路径
/// 上额外分配。
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct PanicEvent<'a> {
    /// 捕到 panic 的守门员名: 当前是 `"guard_sync"` 或 `"guard_async"`。
    /// 稳定; 新的守门员只**新增**字符串, 永不重命名。
    pub site: &'static str,
    /// 从 payload 尽力提取出的 panic 消息。
    pub message: &'a str,
}

/// panic 事件的接收方。实现需要是 `Send + Sync` 且**非阻塞** —— 它运行在
/// panic 发生所在的那个线程上。
pub trait PanicReporter: Send + Sync + 'static {
    /// 每捕到一个 panic 调一次, 在 [`AppError::Internal`] 被回给宿主**之前**。
    ///
    /// [`AppError::Internal`]: archforge_kernel::AppError::Internal
    fn report(&self, event: PanicEvent<'_>);
}

/// 空操作上报器。测试或者日志后端还没接好的环境用。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopReporter;

impl PanicReporter for NoopReporter {
    fn report(&self, _event: PanicEvent<'_>) {}
}

static REPORTER: OnceLock<Box<dyn PanicReporter>> = OnceLock::new();

/// 装上全进程唯一的 panic 上报器。第一次调用胜出; 后续调用会把传入的上报器
/// 原样回 `Err`, 方便调用方释放。推荐调用点: bridge crate 的 `main` 或者
/// FFI 初始化函数。
///
/// # 示例
///
/// ```
/// use archforge_ffi::{install_panic_reporter, NoopReporter};
/// // 进程内第一次调用返回 Ok(())。
/// let _ = install_panic_reporter(NoopReporter);
/// ```
pub fn install_panic_reporter<R: PanicReporter>(reporter: R) -> Result<(), R> {
    // OnceLock::set 失败时会把被拒的值回给你; 我们需要先 box, 所以这里手动
    // 转一下, 把原始的 `R` 保留下来。
    if REPORTER.get().is_some() {
        return Err(reporter);
    }
    let boxed: Box<dyn PanicReporter> = Box::new(reporter);
    // 竞态安全: 并发 install 时落败的一方拿到 Err, 上面那一步已经覆盖了
    // 普通场景。
    REPORTER.set(boxed).map_err(|_| {
        // 此时我们已确认 REPORTER 已被装, 没什么可以原样回给调用方的有效
        // 上报器了。不用 unsafe 凑不出 `R`, 所以 panic —— 而它会被外层守门员
        // 接住。实际上, 上面的 OnceLock 检查已经把这个分支挡掉了。
        unreachable_reporter()
    })
}

fn unreachable_reporter<R: PanicReporter>() -> R {
    // 调用方已确认 REPORTER 已设置; 我们没有什么有意义的值能交回去。不用
    // unsafe 没法凭空生成 `R`, 所以这里 panic —— 反过来又会被外层守门员
    // 接住。实战中上面的 OnceLock 检查就已经避免走到这里了。
    panic!("install_panic_reporter racing install on the same process");
}

pub(crate) fn report_panic(event: PanicEvent<'_>) {
    if let Some(reporter) = REPORTER.get() {
        reporter.report(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // OnceLock 是全进程的; `cargo test` 把每个 #[test] 跑在同一进程的不同
    // 线程里。我们只装一次**带捕获能力**的上报器, 然后跨多个测试通过它来
    // 断言行为。测试顺序不保证, 所以每个测试都要能容忍兄弟测试也产生事件。

    #[derive(Default)]
    struct CapturingReporter {
        events: Mutex<Vec<(&'static str, String)>>,
    }

    impl PanicReporter for CapturingReporter {
        fn report(&self, event: PanicEvent<'_>) {
            self.events
                .lock()
                .unwrap()
                .push((event.site, event.message.to_string()));
        }
    }

    // 装好之后我们还要能在测试里检视上报器, 所以上报器活在静态里;
    // install_panic_reporter 接管的是一个把引用形态的实现转成 trait 对象
    // 的转发器。
    struct Forwarder(&'static CapturingReporter);
    impl PanicReporter for Forwarder {
        fn report(&self, event: PanicEvent<'_>) {
            self.0.report(event)
        }
    }

    static GLOBAL_REPORTER: std::sync::LazyLock<CapturingReporter> =
        std::sync::LazyLock::new(CapturingReporter::default);

    fn ensure_installed() {
        // 跨测试幂等; 第二次 install 拿到的 Err 是良性的。
        let _ = install_panic_reporter(Forwarder(&GLOBAL_REPORTER));
    }

    #[test]
    fn install_is_idempotent() {
        ensure_installed();
        // 第二次 install 既不能崩, 也不能替换掉已装好的上报器。
        let err = install_panic_reporter(NoopReporter);
        assert!(err.is_err(), "second install must be rejected");
    }

    #[test]
    fn reporter_receives_sync_panic() {
        ensure_installed();
        let before = GLOBAL_REPORTER.events.lock().unwrap().len();
        let _ = crate::guard_sync::<u32, _>(|| panic!("report-sync-marker-{}", line!()));
        let events = GLOBAL_REPORTER.events.lock().unwrap();
        assert!(events.len() > before, "reporter must record an event");
        let last = events.last().unwrap();
        assert_eq!(last.0, "guard_sync");
        assert!(last.1.contains("report-sync-marker"));
    }

    #[tokio::test]
    async fn reporter_receives_async_panic() {
        ensure_installed();
        let before = GLOBAL_REPORTER.events.lock().unwrap().len();
        let _ = crate::guard_async::<u32, _>(async { panic!("report-async-marker") }).await;
        let events = GLOBAL_REPORTER.events.lock().unwrap();
        assert!(events.len() > before);
        let entry = events
            .iter()
            .rev()
            .find(|e| e.0 == "guard_async" && e.1.contains("report-async-marker"))
            .expect("async event must be recorded");
        assert_eq!(entry.0, "guard_async");
    }

    #[test]
    fn business_error_does_not_invoke_reporter() {
        ensure_installed();
        let before = GLOBAL_REPORTER.events.lock().unwrap().len();
        let _ = crate::guard_sync::<u32, _>(|| {
            Err(archforge_kernel::AppError::NotFound("nope".into()))
        });
        let after = GLOBAL_REPORTER.events.lock().unwrap().len();
        assert_eq!(before, after, "business error must not trigger reporter");
    }
}
