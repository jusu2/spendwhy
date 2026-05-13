//! 领域层：纯逻辑、纯类型，不依赖 IO，不依赖 FRB。
//!
//! 任何业务规则、计算、状态机都应放在这里，便于在 Rust 内做单元测试。

pub mod fade;
pub mod fragment;
pub mod recovery;

pub use fragment::{Fragment, Stage};
pub use recovery::Recovery;
