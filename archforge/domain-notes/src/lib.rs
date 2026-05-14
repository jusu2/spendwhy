//! # Notes 限界上下文领域模型 (Always-Valid)
//!
//! 富领域聚合 `Note` 留在本 crate 里, 私有字段, 只通过返回 `NoteEvent` 的
//! 方法变更状态。其它层只能看到 [`NoteDto`] (在 `archforge-contract-notes`),
//! 这是 ArchForge 不变量 #2 (只允许 DTO 跨层) 在 Notes 切片的体现。
//!
//! [`NoteDto`]: archforge_contract_notes::NoteDto

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod note;

pub use note::Note;
