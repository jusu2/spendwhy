//! `User` 聚合。

use archforge_contract_auth::{
    DisplayName, Email, PasswordHash, UserCreated, UserDto, UserEvent, UserId, UserPasswordSet,
    UserPasswordVerified, UserRenamed, Version,
};
use archforge_kernel::{AppError, Result, Timestamp};

/// Auth 聚合根。
///
/// 不变式：
/// - `email`、`display_name`、`id` 在构造时即合法
///   （newtype 已在字段层强制有效性）。
/// - 对 [`User::to_dto`] 返回的任何值，`updated_at >= created_at`。
/// - `version` 在变更中单调递增。
/// - 仅通过返回 [`UserEvent`] 的方法进行变更。
///
/// 实现 `Clone` 是为了让 use case 可在变更前后保留不可变快照；
/// 我们刻意不实现 `Serialize` —— 见不变式 #2。
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
    /// 构造一个全新的 `User`。返回聚合及调用方需发布的
    /// `UserCreated` 事件（通常经由 outbox）。
    ///
    /// 创建时 `password_hash` 可选；可在构造后通过 [`Self::set_password`]
    /// 添加。
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

    /// 自持久化重建。DTO 的字段已由 newtype 完成类型校验，
    /// 故此处仅需对时间戳不变式做完整性检查。
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

    /// 投影为传输 DTO。
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

    /// 标识符。
    pub fn id(&self) -> UserId {
        self.id
    }

    /// Email 访问器。
    pub fn email(&self) -> &Email {
        &self.email
    }

    /// 显示名访问器。
    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }

    /// 当前版本。
    pub fn version(&self) -> Version {
        self.version
    }

    /// 是否已存有密码哈希。
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    /// 若存在则借出密码哈希（供 use case 验证使用）。
    pub fn password_hash(&self) -> Option<&PasswordHash> {
        self.password_hash.as_ref()
    }

    /// 领域操作：重命名。
    ///
    /// - 若新名称与当前相同则返回 `Invalid`（拒绝空操作）。
    /// - 若 `now < updated_at` 则返回 `Invalid`（防止时钟漂移）。
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

    /// 领域操作：设置或轮换密码哈希。
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

    /// 领域操作：记录一次成功的认证。
    ///
    /// 该方法本身不变更持久化状态 —— 只发出审计事件。
    ///（失败的登录尝试不属于领域事件：它们属于
    /// 安全监控流。）
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
