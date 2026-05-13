//! Use case: 把当前碎片/恢复列表 + 当前时间 -> 视图模型。
//!
//! 一次 FFI 调用算清楚 fade、growth、列表顺序，避免 Dart 多次回跳 Rust。

use crate::domain::{fade, Fragment, Recovery};
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct HomeViewInput {
    pub fragments: Vec<Fragment>,
    pub recoveries: Vec<Recovery>,
    pub now_ms: i64,
}

#[derive(Debug, Clone)]
pub struct FragmentView {
    pub id: String,
    pub fade_level: f64,
}

#[derive(Debug, Clone)]
pub struct HomeView {
    pub fragments: Vec<FragmentView>,
    pub growth_score: f64,
}

pub fn build_home_view(input: HomeViewInput) -> AppResult<HomeView> {
    for f in &input.fragments {
        f.validate()?;
    }
    for r in &input.recoveries {
        r.validate()?;
    }
    let levels = fade::apply_fade(&input.fragments, &input.recoveries, input.now_ms);
    let fragments = input
        .fragments
        .iter()
        .zip(levels.iter())
        .map(|(f, level)| FragmentView {
            id: f.id.clone(),
            fade_level: *level,
        })
        .collect();
    let growth_score = fade::growth_score(&input.fragments, &input.recoveries, input.now_ms);
    Ok(HomeView {
        fragments,
        growth_score,
    })
}
