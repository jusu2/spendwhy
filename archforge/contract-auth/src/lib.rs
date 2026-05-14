//! # Auth bounded-context 契约
//!
//! 跨越所有 auth 相关 Port 边界的唯一权威来源: 值对象 newtype
//! (`UserId`、`Email`、`DisplayName`、`Version`、`PasswordHash`)、
//! 数据传输对象 (`UserDto`)、命令、查询、领域事件以及 Port trait。
//!
//! 本 crate **仅**依赖 `archforge-kernel` (以及 `PlainPassword` 的
//! zeroising wrapper 所需的 `secrecy`)。它绝不能提及存储技术、传输或
//! 框架 —— 此为不变量 #3。

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod commands;
mod events;
mod port;
mod types;

pub use commands::{
    CreateUserCmd, PlainPassword, RenameUserCmd, SetPasswordCmd, UserQuery, VerifyPasswordCmd,
};
pub use events::{UserCreated, UserEvent, UserPasswordSet, UserPasswordVerified, UserRenamed};
pub use port::{CredentialStore, UserReader, UserRepository, UserWriter};
pub use types::{DisplayName, Email, PasswordHash, UserDto, UserId, Version};
