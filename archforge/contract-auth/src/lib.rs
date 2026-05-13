//! # Auth bounded-context contract
//!
//! The single source of truth for everything that crosses an auth-related
//! Port boundary: value-object newtypes (`UserId`, `Email`, `DisplayName`,
//! `Version`, `PasswordHash`), data-transfer objects (`UserDto`),
//! commands, queries, domain events, and Port traits.
//!
//! This crate depends **only** on `archforge-kernel` (and `secrecy` for
//! `PlainPassword`'s zeroising wrapper). It must never name a storage
//! technology, transport, or framework — that is invariant #3.

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
pub use events::{
    UserCreated, UserEvent, UserPasswordSet, UserPasswordVerified, UserRenamed,
};
pub use port::{CredentialStore, UserReader, UserRepository, UserWriter};
pub use types::{DisplayName, Email, PasswordHash, UserDto, UserId, Version};
