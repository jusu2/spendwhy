//! FFI: 记录恢复事件。

use crate::api::dto::{FragmentDto, RecordRecoveryOutcomeDto, RecoveryDto};
use crate::application::recovery::{record_recovery as run_use_case, RecordRecoveryInput};
use crate::error::AppResult;

#[flutter_rust_bridge::frb(sync)]
pub fn record_recovery(
    recovery: RecoveryDto,
    related_fragments: Vec<FragmentDto>,
) -> anyhow::Result<RecordRecoveryOutcomeDto> {
    let recovery = recovery.into_domain()?;
    let related_fragments = related_fragments
        .into_iter()
        .map(FragmentDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;

    let outcome = run_use_case(RecordRecoveryInput {
        recovery,
        related_fragments,
    })?;

    Ok(RecordRecoveryOutcomeDto {
        schema_version: 1,
        recovery: outcome.recovery.into(),
        fragments_to_advance: outcome.fragments_to_advance,
    })
}
