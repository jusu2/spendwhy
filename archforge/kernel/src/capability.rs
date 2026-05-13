//! Capability marker traits.
//!
//! These are zero-overhead markers that let *use cases* express which
//! capabilities an adapter must provide. The point is to push correctness
//! into the type system: a use case that requires bulk loading cannot be
//! accidentally wired to an adapter that does not support it.
//!
//! ```ignore
//! pub async fn import_users<R>(repo: &R, users: Vec<UserDto>) -> Result<()>
//! where
//!     R: UserWriter + BulkLoadable,
//! { /* ... */ }
//! ```

/// Adapter supports read operations only.
pub trait ReadOnly {}

/// Adapter supports state-mutating writes.
pub trait Writable {}

/// Adapter exposes a transactional unit of work over multiple operations.
pub trait Transactional {}

/// Adapter can accept large batches more efficiently than per-row writes.
pub trait BulkLoadable {}

/// Adapter can deliver events as a long-running stream rather than polling.
pub trait Streamable {}
