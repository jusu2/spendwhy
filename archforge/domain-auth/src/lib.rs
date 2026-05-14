//! # Auth 限界上下文领域模型
//!
//! 富领域、始终有效（Always-Valid）的聚合根 [`User`]。**`User` 类型刻意
//! 不派生 `Serialize`**，仅通过 [`User::to_dto`] 暴露其 DTO 投影。
//! 不变式 #2：领域模型不出 crate。
//!
//! 构造经由 [`User::create`]（新聚合，发出事件）
//! 或 [`User::rehydrate`]（自持久化加载）。变更经由产出对应
//! [`UserEvent`] 的领域方法。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod user;

pub use user::User;
