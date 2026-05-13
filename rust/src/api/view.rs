//! FFI: 复合首页视图。

use crate::api::dto::{
    FragmentDto, FragmentViewDto, HomeViewDto, RecoveryDto, HOME_VIEW_DTO_SCHEMA_VERSION,
};
use crate::application::view;
use crate::error::AppResult;

#[flutter_rust_bridge::frb(sync)]
pub fn build_home_view(
    fragments: Vec<FragmentDto>,
    recoveries: Vec<RecoveryDto>,
    now_ms: i64,
) -> anyhow::Result<HomeViewDto> {
    let fragments = fragments
        .into_iter()
        .map(FragmentDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    let recoveries = recoveries
        .into_iter()
        .map(RecoveryDto::into_domain)
        .collect::<AppResult<Vec<_>>>()?;
    let view = view::build_home_view(view::HomeViewInput {
        fragments,
        recoveries,
        now_ms,
    })?;
    Ok(HomeViewDto {
        schema_version: HOME_VIEW_DTO_SCHEMA_VERSION,
        fragments: view
            .fragments
            .into_iter()
            .map(|f| FragmentViewDto {
                id: f.id,
                fade_level: f.fade_level,
            })
            .collect(),
        growth_score: view.growth_score,
    })
}
