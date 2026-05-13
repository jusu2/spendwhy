//! # Auth bounded-context contract
//!
//! The single source of truth for everything that crosses an auth-related
//! Port boundary: value-object newtypes (`UserId`, `Email`, `DisplayName`),
//! data-transfer objects (`UserDto`), commands (`CreateUserCmd`, …),
//! queries (`UserQuery`), domain events (`UserEvent`), and Port traits
//! (`UserReader`, `UserWriter`).
//!
//! This crate depends **only** on `archforge-kernel`. It must never name a
//! storage technology, transport, or framework — that is invariant #3.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod commands;
mod events;
mod port;
mod types;

pub use commands::{CreateUserCmd, RenameUserCmd, UserQuery};
pub use events::{UserCreated, UserEvent, UserRenamed};
pub use port::{UserReader, UserRepository, UserWriter};
pub use types::{DisplayName, Email, UserDto, UserId};
