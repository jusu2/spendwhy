//! The `User` aggregate.

use archforge_contract_auth::{
    DisplayName, Email, PasswordHash, UserCreated, UserDto, UserEvent, UserId, UserPasswordSet,
    UserPasswordVerified, UserRenamed, Version,
};
use archforge_kernel::{AppError, Result, Timestamp};

/// Auth aggregate root.
///
/// Invariants:
/// - `email`, `display_name`, `id` are always valid by construction
///   (newtypes already enforce field-level validity).
/// - `updated_at >= created_at` for any value returned by [`User::to_dto`].
/// - `version` is monotonic across mutations.
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
    password_hash: Option<PasswordHash>,
    created_at: Timestamp,
    updated_at: Timestamp,
    version: Version,
}

impl User {
    /// Construct a brand-new `User`. Returns the aggregate together with the
    /// `UserCreated` event that callers must publish (typically via outbox).
    ///
    /// `password_hash` is optional at creation; use [`Self::set_password`]
    /// after construction to add one.
    pub fn create(
        email: Email,
        display_name: DisplayName,
        password_hash: Option<PasswordHash>,
        now: Timestamp,
    ) -> (Self, UserEvent) {
        let id = UserId::new();
        let user = Self {
            id,
            email: email.clone(),
            display_name: display_name.clone(),
            password_hash,
            created_at: now,
            updated_at: now,
            version: Version::INITIAL,
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
        if dto.version.as_u64() == 0 {
            return Err(AppError::Invalid(format!(
                "version must be >= 1 for user {}",
                dto.id
            )));
        }
        Ok(Self {
            id: dto.id,
            email: dto.email,
            display_name: dto.display_name,
            password_hash: dto.password_hash,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
            version: dto.version,
        })
    }

    /// Project to a transport DTO.
    pub fn to_dto(&self) -> UserDto {
        UserDto {
            id: self.id,
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            password_hash: self.password_hash.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            schema_version: 1,
        }
    }

    /// Identifier.
    pub fn id(&self) -> UserId {
        self.id
    }

    /// Email accessor.
    pub fn email(&self) -> &Email {
        &self.email
    }

    /// Display name accessor.
    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }

    /// Current version.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Whether a password hash is on file.
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    /// Borrow the password hash if present (for verification by a use case).
    pub fn password_hash(&self) -> Option<&PasswordHash> {
        self.password_hash.as_ref()
    }

    /// Domain operation: rename.
    ///
    /// - `Invalid` if the new name equals the current one (no-op rejection).
    /// - `Invalid` if `now < updated_at` (clock skew protection).
    pub fn rename(&mut self, new_name: DisplayName, now: Timestamp) -> Result<UserEvent> {
        if new_name == self.display_name {
            return Err(AppError::Invalid("display_name unchanged".into()));
        }
        if now < self.updated_at {
            return Err(AppError::Invalid("clock skew: now < updated_at".into()));
        }
        self.display_name = new_name.clone();
        self.updated_at = now;
        self.version = self.version.next();
        Ok(UserEvent::Renamed(UserRenamed {
            id: self.id,
            display_name: new_name,
            at: now,
        }))
    }

    /// Domain operation: set or rotate password hash.
    pub fn set_password(&mut self, hash: PasswordHash, now: Timestamp) -> Result<UserEvent> {
        if now < self.updated_at {
            return Err(AppError::Invalid("clock skew: now < updated_at".into()));
        }
        self.password_hash = Some(hash);
        self.updated_at = now;
        self.version = self.version.next();
        Ok(UserEvent::PasswordSet(UserPasswordSet {
            id: self.id,
            at: now,
        }))
    }

    /// Domain operation: record a successful authentication.
    ///
    /// This does NOT mutate persisted state by itself — it only emits the
    /// audit event. (Login attempts that fail are not domain events: they
    /// belong to a security-monitoring stream.)
    pub fn record_authenticated(&self, now: Timestamp) -> UserEvent {
        UserEvent::PasswordVerified(UserPasswordVerified {
            id: self.id,
            at: now,
        })
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

    fn fake_hash() -> PasswordHash {
        PasswordHash::from_phc("$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAA$BBBBBBBBBBBBBBBB")
            .unwrap()
    }

    #[test]
    fn create_emits_created_event_and_seeds_version() {
        let now = Timestamp::from_ms(100);
        let (u, evt) = User::create(email("a@b"), name("Alice"), None, now);

        assert_eq!(u.display_name().as_str(), "Alice");
        assert_eq!(u.version(), Version::INITIAL);
        let dto = u.to_dto();
        assert_eq!(dto.created_at, now);
        assert_eq!(dto.updated_at, now);
        assert_eq!(dto.schema_version, 1);
        assert_eq!(dto.version, Version::INITIAL);

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
        let (u, _) = User::create(email("a@b"), name("Alice"), None, now);
        let dto = u.to_dto();
        let back = User::rehydrate(dto.clone()).unwrap();
        assert_eq!(back.to_dto(), dto);
    }

    #[test]
    fn rehydrate_rejects_inverted_timestamps() {
        let dto = UserDto {
            id: UserId::new(),
            email: email("a@b"),
            display_name: name("A"),
            password_hash: None,
            created_at: Timestamp::from_ms(200),
            updated_at: Timestamp::from_ms(100),
            version: Version::INITIAL,
            schema_version: 1,
        };
        assert!(matches!(User::rehydrate(dto), Err(AppError::Invalid(_))));
    }

    #[test]
    fn rehydrate_rejects_zero_version() {
        let dto = UserDto {
            id: UserId::new(),
            email: email("a@b"),
            display_name: name("A"),
            password_hash: None,
            created_at: Timestamp::from_ms(0),
            updated_at: Timestamp::from_ms(0),
            version: Version::from_u64(0),
            schema_version: 1,
        };
        assert!(matches!(User::rehydrate(dto), Err(AppError::Invalid(_))));
    }

    #[test]
    fn rename_bumps_version_and_emits_event() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), None, Timestamp::from_ms(100));
        let v0 = u.version();
        let evt = u.rename(name("Bob"), Timestamp::from_ms(200)).unwrap();
        assert_eq!(u.display_name().as_str(), "Bob");
        assert!(u.version() > v0);
        assert_eq!(u.to_dto().updated_at.as_ms(), 200);
        match evt {
            UserEvent::Renamed(r) => assert_eq!(r.display_name.as_str(), "Bob"),
            _ => panic!("expected Renamed"),
        }
    }

    #[test]
    fn rename_rejects_noop() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), None, Timestamp::from_ms(100));
        assert!(matches!(
            u.rename(name("Alice"), Timestamp::from_ms(200)),
            Err(AppError::Invalid(_))
        ));
        assert_eq!(u.version(), Version::INITIAL);
    }

    #[test]
    fn rename_rejects_clock_skew() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), None, Timestamp::from_ms(100));
        assert!(matches!(
            u.rename(name("Bob"), Timestamp::from_ms(50)),
            Err(AppError::Invalid(_))
        ));
    }

    #[test]
    fn set_password_bumps_version_and_emits_event() {
        let (mut u, _) = User::create(email("a@b"), name("Alice"), None, Timestamp::from_ms(100));
        assert!(!u.has_password());
        let evt = u
            .set_password(fake_hash(), Timestamp::from_ms(200))
            .unwrap();
        assert!(u.has_password());
        assert_eq!(u.version(), Version::INITIAL.next());
        match evt {
            UserEvent::PasswordSet(_) => {}
            _ => panic!("expected PasswordSet"),
        }
    }
}
