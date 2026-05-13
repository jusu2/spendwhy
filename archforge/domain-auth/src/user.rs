//! The `User` aggregate.

use archforge_contract_auth::{
    DisplayName, Email, UserCreated, UserDto, UserEvent, UserId, UserRenamed,
};
use archforge_kernel::{AppError, Result, Timestamp};

/// Auth aggregate root.
///
/// Invariants:
/// - `email`, `display_name`, `id`, timestamps are always valid by
///   construction (newtypes already enforce field-level validity).
/// - `updated_at >= created_at` for any value returned by [`User::to_dto`].
/// - Mutation only happens through methods that return a [`UserEvent`].
///
/// `Clone` is implemented so use cases can keep an immutable snapshot
/// before/after a mutation; we deliberately do NOT implement `Serialize` —
/// see invariant #2.
#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    display_name: DisplayName,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl User {
    /// Construct a brand-new `User`. Returns the aggregate together with the
    /// `UserCreated` event that callers must publish (typically via outbox).
    pub fn create(email: Email, display_name: DisplayName, now: Timestamp) -> (Self, UserEvent) {
        let id = UserId::new();
        let user = Self {
            id,
            email: email.clone(),
            display_name: display_name.clone(),
            created_at: now,
            updated_at: now,
        };
        let event = UserEvent::Created(UserCreated {
            id,
            email,
            display_name,
            at: now,
        });
        (user, event)
    }

    /// Reconstitute from persistence. The DTO is already type-validated by
    /// its newtype fields, so the only check left is a sanity timestamp
    /// invariant.
    pub fn rehydrate(dto: UserDto) -> Result<Self> {
        if dto.updated_at < dto.created_at {
            return Err(AppError::Invalid(format!(
                "updated_at({}) < created_at({}) for user {}",
                dto.updated_at.as_ms(),
                dto.created_at.as_ms(),
                dto.id
            )));
        }
        Ok(Self {
            id: dto.id,
            email: dto.email,
            display_name: dto.display_name,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        })
    }

    /// Project to a transport DTO. The schema version emitted is always `1`
    /// for the current `User` layout.
    pub fn to_dto(&self) -> UserDto {
        UserDto {
            id: self.id,
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            schema_version: 1,
        }
    }

    /// Identifier.
    pub fn id(&self) -> UserId {
        self.id
    }

    /// Email accessor (immutable borrow).
    pub fn email(&self) -> &Email {
        &self.email
    }

    /// Display name accessor (immutable borrow).
    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }

    /// Domain operation: rename. Returns the corresponding event on success.
    ///
    /// Returns `AppError::Invalid` if the new name equals the current one —
    /// no-op mutations are not domain operations.
    pub fn rename(&mut self, new_name: DisplayName, now: Timestamp) -> Result<UserEvent> {
        if new_name == self.display_name {
            return Err(AppError::Invalid("display_name unchanged".into()));
        }
        if now < self.updated_at {
            return Err(AppError::Invalid("clock skew: now < updated_at".into()));
        }
        self.display_name = new_name.clone();
        self.updated_at = now;
        Ok(UserEvent::Renamed(UserRenamed {
            id: self.id,
            display_name: new_name,
            at: now,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> DisplayName {
        DisplayName::new(s).unwrap()
    }

    fn email(s: &str) -> Email {
        Email::new(s).unwrap()
    }

    #[test]
    fn create_emits_created_event_and_seeds_timestamps() {
        let now = Timestamp::from_ms(100);
        let (u, evt) = User::create(email("a@b"), name("Alice"), now);

        assert_eq!(u.display_name().as_str(), "Alice");
        let dto = u.to_dto();
        assert_eq!(dto.created_at, now);
        assert_eq!(dto.updated_at, now);
        assert_eq!(dto.schema_version, 1);

        match evt {
            UserEvent::Created(c) => {
                assert_eq!(c.id, dto.id);
                assert_eq!(c.email, dto.email);
                assert_eq!(c.at, now);
            }
            _ => panic!("expected Created"),
        }
    }

    #[test]
    fn rehydrate_round_trips() {
        let now = Timestamp::from_ms(100);
        let (u, _) = User::create(email("a@b"), name("Alice"), now);
        let dto = u.to_dto();
        let back = User::rehydrate(dto.clone()).unwrap();
        assert_eq!(back.to_dto(), dto);
    }

    #[test]
    fn rehydrate_rejects_inverted_timestamps() {
        let mut dto = UserDto {
            id: UserId::new(),
            email: email("a@b"),
            display_name: name("A"),
            created_at: Timestamp::from_ms(200),
            updated_at: Timestamp::from_ms(100),
            schema_version: 1,
        };
        assert!(matches!(
            User::rehydrate(dto.clone()),
            Err(AppError::Invalid(_))
        ));
        dto.updated_at = dto.created_at;
        assert!(User::rehydrate(dto).is_ok());
    }

    #[test]
    fn rename_changes_state_and_emits_event() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), Timestamp::from_ms(100));
        let evt = u.rename(name("Bob"), Timestamp::from_ms(200)).unwrap();
        assert_eq!(u.display_name().as_str(), "Bob");
        assert_eq!(u.to_dto().updated_at.as_ms(), 200);
        match evt {
            UserEvent::Renamed(r) => assert_eq!(r.display_name.as_str(), "Bob"),
            _ => panic!("expected Renamed"),
        }
    }

    #[test]
    fn rename_rejects_noop() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), Timestamp::from_ms(100));
        assert!(matches!(
            u.rename(name("Alice"), Timestamp::from_ms(200)),
            Err(AppError::Invalid(_))
        ));
    }

    #[test]
    fn rename_rejects_clock_skew() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), Timestamp::from_ms(100));
        assert!(matches!(
            u.rename(name("Bob"), Timestamp::from_ms(50)),
            Err(AppError::Invalid(_))
        ));
    }
}
