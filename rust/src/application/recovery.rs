//! Use case: 记录一次恢复事件，并返回需要被推进到 recovery 阶段的碎片 ID。
//!
//! 这条业务规则曾经写在 Dart `FragmentsProvider.addRecovery` 里，
//! 现在迁回 Rust 应用层，作为唯一权威实现。
//!
//! ADR-0003 之后：所有入参都已是合法值对象，本层只关心业务规则。

use crate::domain::{Fragment, FragmentId, Recovery, Stage};
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
    pub fragments_to_advance: Vec<FragmentId>,
}

pub fn record_recovery(input: RecordRecoveryInput) -> AppResult<RecordRecoveryOutcome> {
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
        Fragment::try_new(id, 0, 3, 270, stage).expect("valid fragment")
    }

    fn recovery(id: &str, related: &[&str]) -> Recovery {
        Recovery::try_new(
            id,
            100,
            3,
            "felt better",
            related.iter().map(|s| (*s).to_string()).collect(),
        )
        .expect("valid recovery")
    }

    #[test]
    fn outburst_fragments_advance() {
        let outcome = record_recovery(RecordRecoveryInput {
            recovery: recovery("r1", &["a", "b"]),
            related_fragments: vec![
                fragment("a", Stage::Outburst),
                fragment("b", Stage::Recovery),
            ],
        })
        .expect("should succeed");
        assert_eq!(outcome.fragments_to_advance.len(), 1);
        assert_eq!(outcome.fragments_to_advance[0].as_str(), "a");
    }

    #[test]
    fn invalid_recovery_rejected_at_parse_time() {
        // 通过 try_new 解析空 id 时即拒绝；application 层无需再校验。
        assert!(Recovery::try_new("", 0, 3, "x", vec![]).is_err());
    }
}
