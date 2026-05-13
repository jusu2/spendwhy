//! Capability marker traits.
//!
//! Zero-overhead markers that let *use cases* express which capabilities an
//! adapter must provide. The point is to push correctness into the type
//! system: a use case that requires bulk loading cannot be accidentally
//! wired to an adapter that does not support it.
//!
//! ```ignore
//! pub async fn import_users<R>(repo: &R, ctx: &Context, users: Vec<UserDto>) -> Result<usize>
//! where
//!     R: UserWriter + BulkLoadable,
//! { /* ... */ }
//!
//! // Adapter declares the capability:
//! impl BulkLoadable for SqliteUserRepo {}
//!
//! // The InMemory adapter does NOT impl BulkLoadable, so:
//! //    import_users(&memory_repo, &ctx, vec![...]).await
//! // fails to compile — the type system rejects miswiring.
//! ```
//!
//! These markers carry no methods on purpose: the **business** capability
//! lives in the Port traits in `contract-*` crates; the marker simply
//! indicates that an adapter has opted into the relevant performance /
//! semantic guarantees (transactional, batchable, streamable, …). The
//! consequence is that adding a marker is a one-line, non-breaking change.

/// Adapter supports read operations only.
pub trait ReadOnly {}

/// Adapter supports state-mutating writes.
pub trait Writable {}

/// Adapter exposes a transactional unit of work over multiple operations.
///
/// Bound on this when a use case needs all-or-nothing semantics across
/// several Port calls.
pub trait Transactional {}

/// Adapter can accept large batches more efficiently than per-row writes.
///
/// Bound on this for `import_*` / `bulk_*` use cases.
pub trait BulkLoadable {}

/// Adapter can deliver events as a long-running stream rather than polling.
///
/// Bound on this for projector / read-model maintenance use cases.
pub trait Streamable {}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time check: the markers can be used as super-trait bounds.
    fn _accepts_writable<T: Writable>(_: &T) {}
    fn _accepts_bulk<T: Writable + BulkLoadable>(_: &T) {}

    struct Toy;
    impl Writable for Toy {}
    impl BulkLoadable for Toy {}

    #[test]
    fn capabilities_compose() {
        let t = Toy;
        _accepts_writable(&t);
        _accepts_bulk(&t);
    }
}
