//! # In-memory auth adapter
//!
//! `DashMap`-backed implementation of `UserReader + UserWriter +
//! CredentialStore`, plus an in-process [`OutboxSink`].
//!
//! Atomicity: insert/update/delete use [`DashMap::entry`] so the
//! check-then-write pair is a single critical section. Concurrent attempts
//! to insert the same email or update the same id therefore see one
//! winner and one [`AppError::Conflict`], never a torn state.
//!
//! Capability markers: [`Writable`] and [`BulkLoadable`].

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{
    CredentialStore, Email, PasswordHash, UserDto, UserEvent, UserId, UserReader, UserWriter,
    Version,
};
use archforge_kernel::{
    AppError, BulkLoadable, Context, DomainEvent, OutboxSink, Result, Writable,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::{Arc, Mutex};

/// In-memory auth repository.
#[derive(Clone, Default)]
pub struct InMemoryUserRepo {
    by_id: Arc<DashMap<UserId, UserDto>>,
    by_email: Arc<DashMap<Email, UserId>>,
}

impl InMemoryUserRepo {
    /// Fresh, empty repository.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored users (mostly for tests/diagnostics).
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// `true` iff no users are stored.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl Writable for InMemoryUserRepo {}
impl BulkLoadable for InMemoryUserRepo {}

#[async_trait]
impl UserReader for InMemoryUserRepo {
    async fn find_by_id(&self, _ctx: &Context, id: &UserId) -> Result<Option<UserDto>> {
        Ok(self.by_id.get(id).map(|e| e.value().clone()))
    }

    async fn find_by_email(&self, _ctx: &Context, email: &Email) -> Result<Option<UserDto>> {
        let id = match self.by_email.get(email) {
            Some(e) => *e.value(),
            None => return Ok(None),
        };
        Ok(self.by_id.get(&id).map(|e| e.value().clone()))
    }
}

#[async_trait]
impl UserWriter for InMemoryUserRepo {
    async fn insert(&self, _ctx: &Context, user: &UserDto) -> Result<()> {
        if user.version != Version::INITIAL {
            return Err(AppError::Invalid(format!(
                "insert: expected Version::INITIAL, got {}",
                user.version
            )));
        }

        // Reserve email first via `entry` — a single critical section that
        // both checks for presence AND inserts. Prevents the
        // contains_key + insert TOCTOU.
        use dashmap::mapref::entry::Entry;
        let email_entry = self.by_email.entry(user.email.clone());
        if matches!(email_entry, Entry::Occupied(_)) {
            return Err(AppError::Conflict(format!("email exists: {}", user.email)));
        }

        // Now reserve the id slot.
        let id_entry = self.by_id.entry(user.id);
        if matches!(id_entry, Entry::Occupied(_)) {
            // We have NOT yet inserted the email — releasing `email_entry`
            // is a no-op because we only borrowed the vacant slot.
            return Err(AppError::Conflict(format!("id exists: {}", user.id)));
        }

        // Both slots vacant — commit.
        email_entry.insert_entry(user.id);
        id_entry.insert_entry(user.clone());
        Ok(())
    }

    async fn update(
        &self,
        _ctx: &Context,
        user: &UserDto,
        expected_version: Version,
    ) -> Result<()> {
        // Hold the `entry` to the id row for the entire critical section.
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(user.id) {
            Entry::Vacant(_) => Err(AppError::NotFound(format!("user {}", user.id))),
            Entry::Occupied(mut existing_slot) => {
                let existing = existing_slot.get().clone();
                if existing.version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for user {}: expected {}, found {}",
                        user.id, expected_version, existing.version
                    )));
                }
                if user.version <= expected_version {
                    return Err(AppError::Invalid(format!(
                        "update: new version {} must be strictly greater than expected {}",
                        user.version, expected_version
                    )));
                }

                if existing.email != user.email {
                    // Reserve the new email slot atomically before swapping.
                    match self.by_email.entry(user.email.clone()) {
                        Entry::Occupied(holder) if *holder.get() != user.id => {
                            return Err(AppError::Conflict(format!(
                                "email exists: {}",
                                user.email
                            )));
                        }
                        Entry::Occupied(_) => {
                            // Same id (unreachable in practice but harmless).
                        }
                        Entry::Vacant(slot) => {
                            slot.insert_entry(user.id);
                        }
                    }
                    self.by_email.remove(&existing.email);
                }
                existing_slot.insert(user.clone());
                Ok(())
            }
        }
    }

    async fn delete(&self, _ctx: &Context, id: &UserId, expected_version: Version) -> Result<()> {
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(*id) {
            Entry::Vacant(_) => Ok(()), // idempotent delete
            Entry::Occupied(slot) => {
                if slot.get().version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for user {}: expected {}, found {}",
                        id,
                        expected_version,
                        slot.get().version
                    )));
                }
                let dto = slot.remove();
                self.by_email.remove(&dto.email);
                Ok(())
            }
        }
    }
}

#[async_trait]
impl CredentialStore for InMemoryUserRepo {
    async fn set_password(
        &self,
        ctx: &Context,
        id: &UserId,
        hash: &PasswordHash,
        expected_version: Version,
    ) -> Result<()> {
        use dashmap::mapref::entry::Entry;
        match self.by_id.entry(*id) {
            Entry::Vacant(_) => Err(AppError::NotFound(format!("user {}", id))),
            Entry::Occupied(mut slot) => {
                if slot.get().version != expected_version {
                    return Err(AppError::Conflict(format!(
                        "version mismatch for user {}: expected {}, found {}",
                        id,
                        expected_version,
                        slot.get().version
                    )));
                }
                let mut dto = slot.get().clone();
                dto.password_hash = Some(hash.clone());
                dto.version = expected_version.next();
                slot.insert(dto);
                let _ = ctx;
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// In-memory outbox.
// ---------------------------------------------------------------------------

/// In-memory implementation of [`OutboxSink`].
///
/// Records each appended event in a `Vec` for inspection by tests. Suitable
/// for unit tests; production deployments should use a durable outbox
/// (sqlite/kafka/rabbit).
#[derive(Clone, Default)]
pub struct InMemoryOutbox {
    inner: Arc<Mutex<Vec<RecordedEvent>>>,
}

/// Snapshot of a recorded event.
#[derive(Debug, Clone)]
pub struct RecordedEvent {
    /// Stable type identifier.
    pub event_type: &'static str,
    /// Aggregate identifier.
    pub aggregate_id: String,
    /// Wall-clock time the aggregate said this happened.
    pub occurred_at_ms: i64,
}

impl InMemoryOutbox {
    /// Empty outbox.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all recorded events.
    pub fn snapshot(&self) -> Vec<RecordedEvent> {
        self.inner.lock().expect("outbox poisoned").clone()
    }
}

#[async_trait]
impl OutboxSink for InMemoryOutbox {
    async fn append(&self, _ctx: &Context, event: &dyn DomainEvent) -> Result<()> {
        let rec = RecordedEvent {
            event_type: event.event_type(),
            aggregate_id: event.aggregate_id(),
            occurred_at_ms: event.occurred_at().as_ms(),
        };
        self.inner.lock().expect("outbox poisoned").push(rec);
        Ok(())
    }
}

/// Re-export the event type so external test harnesses can assert on the
/// event variants without pulling in the contract crate explicitly.
pub use archforge_contract_auth::UserEvent as ReExportedUserEvent;
// Suppress unused-import warning when the re-export isn't pulled.
#[allow(dead_code)]
fn _force_user_event_in_scope(_e: &UserEvent) {}

#[cfg(test)]
mod conformance_tests {
    use super::{InMemoryOutbox, InMemoryUserRepo};
    use archforge_kernel::{DomainEvent, OutboxSink, Timestamp};

    struct DummyEvent;
    impl DomainEvent for DummyEvent {
        fn event_type(&self) -> &'static str {
            "test.dummy.v1"
        }
        fn aggregate_id(&self) -> String {
            "agg".into()
        }
        fn occurred_at(&self) -> Timestamp {
            Timestamp::from_ms(0)
        }
    }

    #[tokio::test]
    async fn passes_port_conformance() {
        archforge_conformance::user_repo_conformance(|| async { InMemoryUserRepo::new() }).await;
    }

    #[tokio::test]
    async fn passes_concurrency_conformance() {
        archforge_conformance::user_repo_concurrency_conformance(|| async {
            InMemoryUserRepo::new()
        })
        .await;
    }

    #[tokio::test]
    async fn outbox_records_appended_events() {
        let o = InMemoryOutbox::new();
        let ctx = archforge_kernel::Context::test();
        OutboxSink::append(&o, &ctx, &DummyEvent).await.unwrap();
        assert_eq!(o.snapshot().len(), 1);
        assert_eq!(o.snapshot()[0].event_type, "test.dummy.v1");
    }
}
