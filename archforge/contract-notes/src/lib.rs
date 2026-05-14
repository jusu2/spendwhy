//! # Notes 限界上下文契约
//!
//! 一切跨越 Notes 相关 Port 边界的东西的**唯一信源**: 值对象 newtype
//! (`NoteId`、`Title`、`Body`、`Tag`、`Version`)、DTO (`NoteDto`)、命令、
//! 领域事件、Port trait。
//!
//! 本 crate **只**依赖 `archforge-kernel`。它绝不命名任何存储技术、传输或
//! 框架 —— 这是 ArchForge 不变量 #3。
//!
//! ## 范围 (Phase 1A)
//!
//! 当前为**只命令侧**: Create / Edit / Archive / Restore。读侧 (查询、
//! 投影) 留待后续 phase。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod commands;
mod events;
mod port;
mod types;

pub use commands::{ArchiveNoteCmd, CreateNoteCmd, EditNoteCmd, RestoreNoteCmd};
pub use events::{NoteArchived, NoteCreated, NoteEdited, NoteEvent, NoteRestored};
pub use port::{NoteReader, NoteRepository, NoteWriter};
pub use types::{Body, NoteDto, NoteId, NoteStatus, Tag, Title, Version};
