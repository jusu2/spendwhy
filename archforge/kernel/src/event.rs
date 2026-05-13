//! Domain event primitives.
//!
//! `DomainEvent` is the smallest contract every aggregate-emitted fact must
//! satisfy. `OutboxSink` is the Port adapters implement to persist or publish
//! those facts; concrete outbox implementations (file, sqlite, kafka) live in
//! `infra-*` crates.

use crate::{Context, Result, Timestamp};
use async_trait::async_trait;

/// A fact produced by an aggregate root.
///
/// Implementations are concrete `struct`s or `enum`s that live in
/// `contract-*` crates so they can be serialised across process boundaries.
pub trait DomainEvent: Send + Sync + 'static {
    /// Stable, versioned event type identifier, e.g. `"auth.user.created.v1"`.
    ///
    /// **Must never change** for an existing variant; introduce a new event
    /// type for breaking schema changes.
    fn event_type(&self) -> &'static str;

    /// Aggregate root identifier this event belongs to. Used for partitioning
    /// in the outbox and for replay routing.
    fn aggregate_id(&self) -> String;

    /// When the event happened. Adapters must NOT overwrite this with their
    /// own clock — the aggregate decided.
    fn occurred_at(&self) -> Timestamp;
}

/// Port: durable sink for domain events.
///
/// Implementations are expected to be at-least-once and idempotent against
/// `(event_type, aggregate_id, occurred_at)` triples.
#[async_trait]
pub trait OutboxSink: Send + Sync {
    /// Append a single event. The sink is responsible for durability before
    /// returning `Ok`.
    async fn append(&self, ctx: &Context, event: &dyn DomainEvent) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppError;

    struct DummyEvent {
        id: String,
        at: Timestamp,
    }

    impl DomainEvent for DummyEvent {
        fn event_type(&self) -> &'static str {
            "test.dummy.v1"
        }
        fn aggregate_id(&self) -> String {
            self.id.clone()
        }
        fn occurred_at(&self) -> Timestamp {
            self.at
        }
    }

    #[test]
    fn domain_event_is_dyn_safe() {
        // Compile-time check: `&dyn DomainEvent` must be usable in trait
        // objects (e.g. `OutboxSink::append(&self, _, &dyn DomainEvent)`).
        // If this signature ever requires `Sized`, the kernel breaks.
        fn _accept_dyn(evt: &dyn DomainEvent) -> &'static str {
            evt.event_type()
        }

        let e = DummyEvent {
            id: "agg-1".into(),
            at: Timestamp::from_ms(123),
        };
        assert_eq!(_accept_dyn(&e), "test.dummy.v1");
        assert_eq!(e.aggregate_id(), "agg-1");
        assert_eq!(e.occurred_at().as_ms(), 123);
    }

    // Compile-time check: AppError flows through.
    #[allow(dead_code)]
    fn _err_type_smoke() -> Result<()> {
        Err(AppError::Internal("x".into()))
    }
}
