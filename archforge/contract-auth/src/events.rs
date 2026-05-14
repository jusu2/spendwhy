//! auth 聚合产出的领域事件。
//!
//! 每个事件变体都有稳定的、带版本的判别 (`v` tag), 是 wire 契约的一部分。
//! 重命名或 schema 破坏只能通过引入新变体来表达, 而不能改动已有变体 ——
//! 这是让旧消费者继续工作的唯一办法。

use crate::types::{DisplayName, Email, UserId};
use archforge_kernel::{DomainEvent, Timestamp};
use serde::{Deserialize, Serialize};

/// auth 聚合产出的所有领域事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "v")]
pub enum UserEvent {
    /// `auth.user.created.v1`。
    #[serde(rename = "auth.user.created.v1")]
    Created(UserCreated),
    /// `auth.user.renamed.v1`。
    #[serde(rename = "auth.user.renamed.v1")]
    Renamed(UserRenamed),
    /// `auth.user.password_set.v1`。
    #[serde(rename = "auth.user.password_set.v1")]
    PasswordSet(UserPasswordSet),
    /// `auth.user.password_verified.v1`。
    ///
    /// 认证成功时发出。失败尝试**不**作为领域事件发出 —— 它们属于这个
    /// 聚合之外的安全审计流。
    #[serde(rename = "auth.user.password_verified.v1")]
    PasswordVerified(UserPasswordVerified),
}

/// `auth.user.created.v1` 的 payload。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserCreated {
    /// 新用户 id。
    pub id: UserId,
    /// 创建时的邮箱。
    pub email: Email,
    /// 创建时的展示名。
    pub display_name: DisplayName,
    /// 发生时间。
    pub at: Timestamp,
}

/// `auth.user.renamed.v1` 的 payload。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserRenamed {
    /// 被重命名用户的 id。
    pub id: UserId,
    /// 重命名后的新展示名。
    pub display_name: DisplayName,
    /// 发生时间。
    pub at: Timestamp,
}

/// `auth.user.password_set.v1` 的 payload。不携带 hash —— 审计日志只需
/// 知道密码*被设置了*, 不需要知道值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPasswordSet {
    /// 密码变更的用户。
    pub id: UserId,
    /// 发生时间。
    pub at: Timestamp,
}

/// `auth.user.password_verified.v1` 的 payload。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPasswordVerified {
    /// 通过认证的用户。
    pub id: UserId,
    /// 发生时间。
    pub at: Timestamp,
}

impl DomainEvent for UserEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "auth.user.created.v1",
            Self::Renamed(_) => "auth.user.renamed.v1",
            Self::PasswordSet(_) => "auth.user.password_set.v1",
            Self::PasswordVerified(_) => "auth.user.password_verified.v1",
        }
    }

    fn aggregate_id(&self) -> String {
        match self {
            Self::Created(e) => e.id.to_string(),
            Self::Renamed(e) => e.id.to_string(),
            Self::PasswordSet(e) => e.id.to_string(),
            Self::PasswordVerified(e) => e.id.to_string(),
        }
    }

    fn occurred_at(&self) -> Timestamp {
        match self {
            Self::Created(e) => e.at,
            Self::Renamed(e) => e.at,
            Self::PasswordSet(e) => e.at,
            Self::PasswordVerified(e) => e.at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Email;

    #[test]
    fn created_event_round_trips() {
        let e = UserEvent::Created(UserCreated {
            id: UserId::new(),
            email: Email::new("a@b").unwrap(),
            display_name: DisplayName::new("A").unwrap(),
            at: Timestamp::from_ms(1),
        });
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("auth.user.created.v1"));
        let back: UserEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn event_type_string_is_stable() {
        let e = UserEvent::Renamed(UserRenamed {
            id: UserId::new(),
            display_name: DisplayName::new("B").unwrap(),
            at: Timestamp::from_ms(2),
        });
        assert_eq!(e.event_type(), "auth.user.renamed.v1");
    }

    #[test]
    fn password_set_event_round_trips() {
        let e = UserEvent::PasswordSet(UserPasswordSet {
            id: UserId::new(),
            at: Timestamp::from_ms(99),
        });
        assert_eq!(e.event_type(), "auth.user.password_set.v1");
        let s = serde_json::to_string(&e).unwrap();
        let back: UserEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }
}
