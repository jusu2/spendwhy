//! FFI: fade 与 growth_score 计算入口。
//!
//! 保留这两个细粒度入口用于轻量调用（例如详情页只需要单条 fade）。
//! 复合视图请使用 [`crate::api::view::build_home_view`] 一次返回。

use crate::api::dto::{FragmentDto, RecoveryDto};
use crate::domain::fade;
use crate::error::AppResult;

#[flutter_rust_bridge::frb(sync)]
pub fn fade_level(
    fragment: FragmentDto,
    recoveries: Vec<RecoveryDto>,
    now_ms: i64,
) -> anyhow::Result<f64> {
    let f = fragment.into_domain()?;
    let recoveries = recoveries
        .into_iter()
        .map(RecoveryDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    Ok(fade::fade_level(&f, &recoveries, now_ms))
}

#[flutter_rust_bridge::frb(sync)]
pub fn growth_score(
    fragments: Vec<FragmentDto>,
    recoveries: Vec<RecoveryDto>,
    now_ms: i64,
) -> anyhow::Result<f64> {
    let fragments = fragments
        .into_iter()
        .map(FragmentDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    let recoveries = recoveries
        .into_iter()
        .map(RecoveryDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    Ok(fade::growth_score(&fragments, &recoveries, now_ms))
}

/// 提供给 UI 成长曲线使用的批量采样接口。
///
/// 返回 `samples + 1` 个点，代替 Dart 侧重复实现同一算法。
#[flutter_rust_bridge::frb(sync)]
pub fn growth_score_series(
    fragments: Vec<FragmentDto>,
    recoveries: Vec<RecoveryDto>,
    start_ms: i64,
    end_ms: i64,
    samples: u32,
) -> anyhow::Result<Vec<f64>> {
    let fragments = fragments
        .into_iter()
        .map(FragmentDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    let recoveries = recoveries
        .into_iter()
        .map(RecoveryDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    Ok(fade::growth_score_series(
        &fragments,
        &recoveries,
        start_ms,
        end_ms,
        samples,
    ))
}
