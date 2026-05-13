//! FRB 初始化入口。
//!
//! 此前的 `greet` 演示函数已删除，避免示例 API 出现在生产桥接表面。

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
