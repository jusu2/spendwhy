//! # Auth bounded-context domain model
//!
//! Rich, Always-Valid aggregate root [`User`]. **The `User` type intentionally
//! never derives `Serialize`** and exposes only its DTO projection through
//! [`User::to_dto`]. Invariant #2: domain models stay inside their crate.
//!
//! Construction goes through [`User::create`] (new aggregate, emits event)
//! or [`User::rehydrate`] (load from persistence). Mutation goes through
//! domain methods that produce the corresponding [`UserEvent`].

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

mod user;

pub use user::User;
