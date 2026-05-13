//! 应用层：组合领域规则，提供 use case。
//!
//! - 不直接 IO；输入是已经准备好的领域对象。
//! - 输出是确定性结果或 `AppError`。

pub mod recovery;
pub mod view;
