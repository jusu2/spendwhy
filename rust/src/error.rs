//! AppError / AppResult 重新导出，让非 api 层使用同一份错误模型。
//!
//! 真正的定义在 `crate::api::error`，因为 flutter_rust_bridge 需要扫描定义点。

pub use crate::api::error::{AppError, AppResult};
