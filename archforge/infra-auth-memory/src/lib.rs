//! # In-memory auth adapter
//!
//! `DashMap`-backed implementation of `UserReader + UserWriter`. Ideal for
//! tests and UI prototyping; not suitable for production unless paired with
//! an outer durability layer.
//!
//! Carries the [`Writable`] capability marker.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms, missing_docs)]
#![deny(unused_must_use)]

use archforge_contract_auth::{Email, UserDto, UserId, UserReader, UserWriter};
use archforge_kernel::{AppError, Context, Result, Writable};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

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
}

impl Writable for InMemoryUserRepo {}

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
        if self.by_email.contains_key(&user.email) {
            return Err(AppError::Conflict(format!("email exists: {}", user.email)));
        }
        if self.by_id.contains_key(&user.id) {
            return Err(AppError::Conflict(format!("id exists: {}", user.id)));
        }
        self.by_email.insert(user.email.clone(), user.id);
        self.by_id.insert(user.id, user.clone());
        Ok(())
    }

    async fn update(&self, _ctx: &Context, user: &UserDto) -> Result<()> {
        let existing = self
            .by_id
            .get(&user.id)
            .map(|e| e.value().clone())
            .ok_or_else(|| AppError::NotFound(format!("user {}", user.id)))?;

        if existing.email != user.email {
            // Email is changing — verify the new one isn't taken by *another* row.
            if let Some(holder) = self.by_email.get(&user.email) {
                if *holder.value() != user.id {
                    return Err(AppError::Conflict(format!("email exists: {}", user.email)));
                }
            }
            self.by_email.remove(&existing.email);
            self.by_email.insert(user.email.clone(), user.id);
        }
        self.by_id.insert(user.id, user.clone());
        Ok(())
    }
}

#[cfg(test)]
mod conformance_tests {
    use super::InMemoryUserRepo;

    #[tokio::test]
    async fn passes_port_conformance() {
        archforge_conformance::user_repo_conformance(|| async { InMemoryUserRepo::new() }).await;
    }
}
