//! Use case: 记录一次恢复事件，并返回需要被推进到 recovery 阶段的碎片 ID。
//!
//! 这条业务规则曾经写在 Dart `FragmentsProvider.addRecovery` 里，
//! 现在迁回 Rust 应用层，作为唯一权威实现。

use crate::domain::{Fragment, Recovery, Stage};
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct RecordRecoveryInput {
    pub recovery: Recovery,
    /// 与 `recovery.related_fragment_ids` 对应的当前碎片快照。
    /// 调用方负责把当前已知的碎片状态喂进来。
    pub related_fragments: Vec<Fragment>,
}

#[derive(Debug, Clone)]
pub struct RecordRecoveryOutcome {
    pub recovery: Recovery,
    /// 需要被持久化为 `recovery` 阶段的碎片 ID。
    pub fragments_to_advance: Vec<String>,
}

pub fn record_recovery(input: RecordRecoveryInput) -> AppResult<RecordRecoveryOutcome> {
    input.recovery.validate()?;
    for f in &input.related_fragments {
        f.validate()?;
    }

    let mut fragments_to_advance = Vec::new();
    for f in &input.related_fragments {
        if input
            .recovery
            .related_fragment_ids
            .iter()
            .any(|id| id == &f.id)
            && f.stage == Stage::Outburst
        {
            fragments_to_advance.push(f.id.clone());
        }
    }

    Ok(RecordRecoveryOutcome {
        recovery: input.recovery,
        fragments_to_advance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Stage;

    fn fragment(id: &str, stage: Stage) -> Fragment {
        Fragment {
            id: id.into(),
            created_at_ms: 0,
            intensity: 3,
            fade_period_days: 270,
            stage,
        }
    }

    #[test]
    fn outburst_fragments_advance() {
        let outcome = record_recovery(RecordRecoveryInput {
            recovery: Recovery {
                id: "r1".into(),
                created_at_ms: 100,
                intensity: 3,
                description: "felt better".into(),
                related_fragment_ids: vec!["a".into(), "b".into()],
            },
            related_fragments: vec![
                fragment("a", Stage::Outburst),
                fragment("b", Stage::Recovery),
            ],
        })
        .expect("should succeed");
        assert_eq!(outcome.fragments_to_advance, vec!["a".to_string()]);
    }

    #[test]
    fn invalid_recovery_rejected() {
        let err = record_recovery(RecordRecoveryInput {
            recovery: Recovery {
                id: "".into(),
                created_at_ms: 0,
                intensity: 3,
                description: "x".into(),
                related_fragment_ids: vec![],
            },
            related_fragments: vec![],
        })
        .unwrap_err();
        assert_eq!(err.kind, crate::error::AppError::KIND_INVALID_INPUT);
    }
}
