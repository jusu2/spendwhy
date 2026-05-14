//! 领域事件原语。
//!
//! `DomainEvent` 是每个聚合根产出的事实必须满足的最小契约。
//! `OutboxSink` 是 adapter 实现的 Port, 用来持久化或发布这些事实;
//! 具体 outbox 实现 (file、sqlite、kafka) 在 `infra-*` crate 中。

use crate::{Context, Result, Timestamp};
use async_trait::async_trait;

/// 由聚合根产出的事实。
///
/// 实现是具体的 `struct` 或 `enum`, 放在 `contract-*` crate 里, 以便能跨
/// 进程边界序列化。
pub trait DomainEvent: Send + Sync + 'static {
    /// 稳定的、带版本的事件类型标识符, 如 `"auth.user.created.v1"`。
    ///
    /// 对已有变体**永不可变**; schema 破坏性变更要引入新的事件类型。
    fn event_type(&self) -> &'static str;

    /// 事件所属的聚合根标识符。用于 outbox 中的分区与回放路由。
    fn aggregate_id(&self) -> String;

    /// 事件发生的时间。Adapter 不得用自己的时钟覆盖它 —— 由聚合决定。
    fn occurred_at(&self) -> Timestamp;
}

/// Port: 领域事件的持久 sink。
///
/// 实现应是 at-least-once, 并对 `(event_type, aggregate_id, occurred_at)`
/// 三元组幂等。
#[async_trait]
pub trait OutboxSink: Send + Sync {
    /// 追加单个事件。返回 `Ok` 前 sink 负责持久化。
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
        // 编译期检查: `&dyn DomainEvent` 必须能用在 trait object 中
        // (如 `OutboxSink::append(&self, _, &dyn DomainEvent)`)。
        // 一旦这个签名要求 `Sized`, kernel 就坏了。
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

    // 编译期检查: AppError 类型流通正常。
    #[allow(dead_code)]
    fn _err_type_smoke() -> Result<()> {
        Err(AppError::Internal("x".into()))
    }
}
