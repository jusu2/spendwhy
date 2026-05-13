//! 场景关键词: 启动初始化 / 进程级单例 / `#[frb(init)]` / 热重启 → 选我。
//!
//! 模式 P: 单例服务 + 显式 shutdown。
//!
//! - 启动时初始化运行时信息、事件总线、连接池。
//! - 用 `OnceLock` 而非全局可变变量。
//! - 不要在这里做"昂贵"工作 (DB 迁移、远程拉取); 那些应是显式异步函数。
//! - Flutter hot-restart **不会** 重新加载 dylib, 所以 `#[frb(init)]` 也不会重跑;
//!   需要 reset 状态时, Dart 侧显式调 `transport_shutdown()` 再做下一步。

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use flutter_rust_bridge::frb;

#[derive(Debug, Clone)]
pub struct TransportRuntimeInfo {
    pub started_at_ms: i64,
    pub version: String,
    pub running: bool,
}

static RUNTIME: OnceLock<RuntimeCell> = OnceLock::new();

struct RuntimeCell {
    started_at_ms: AtomicI64,
    version: String,
    running: AtomicBool,
}

fn cell() -> &'static RuntimeCell {
    RUNTIME.get_or_init(|| RuntimeCell {
        started_at_ms: AtomicI64::new(0),
        version: env!("CARGO_PKG_VERSION").to_string(),
        running: AtomicBool::new(false),
    })
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// FRB 启动钩子: app 首次调用 Rust 时执行一次。
#[frb(init)]
pub fn transport_init() {
    let c = cell();
    c.started_at_ms.store(now_ms(), Ordering::SeqCst);
    c.running.store(true, Ordering::SeqCst);
}

/// Dart 端显式调; 用于 hot-restart 或测试间隔离。
/// 这里只重置标志位; 真实业务应在此 flush 数据 / 关闭事件总线 / 关闭文件句柄。
pub async fn transport_shutdown() {
    let c = cell();
    c.running.store(false, Ordering::SeqCst);
    c.started_at_ms.store(0, Ordering::SeqCst);
}

#[frb(sync)]
pub fn transport_runtime_info() -> TransportRuntimeInfo {
    let c = cell();
    TransportRuntimeInfo {
        started_at_ms: c.started_at_ms.load(Ordering::SeqCst),
        version: c.version.clone(),
        running: c.running.load(Ordering::SeqCst),
    }
}

#[frb(sync)]
pub fn transport_runtime_started_at_ms() -> i64 {
    cell().started_at_ms.load(Ordering::SeqCst)
}

#[frb(sync)]
pub fn transport_runtime_version() -> String {
    cell().version.clone()
}
