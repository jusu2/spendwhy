//! 淡化与和解分数算法。
//!
//! 纯函数：相同输入相同输出，不依赖时间外部源。`now_ms` 由调用方传入。

use super::{Fragment, Recovery};

/// 单条碎片清晰度 0..=1（1 = 清晰，0 = 几乎不可见）
pub fn fade_level(fragment: &Fragment, recoveries: &[Recovery], now_ms: i64) -> f64 {
    let age_days = days_between(now_ms, fragment.created_at.ms());
    let time_factor = (age_days / f64::from(fragment.fade_period_days.days())).clamp(0.0, 1.0);

    let mut recovery_factor = 0.0_f64;
    for r in recoveries {
        if !r.related_fragment_ids.iter().any(|id| id == &fragment.id) {
            continue;
        }
        let since = days_between(now_ms, r.created_at.ms()).max(0.0);
        let time_weight = (-since / 60.0).exp();
        recovery_factor += (f64::from(r.intensity.value()) / 5.0) * 0.25 * time_weight;
    }

    (1.0 - time_factor - recovery_factor).clamp(0.0, 1.0)
}

/// 批量计算 fade_level，顺序与输入一致。
pub fn apply_fade(fragments: &[Fragment], recoveries: &[Recovery], now_ms: i64) -> Vec<f64> {
    fragments
        .iter()
        .map(|f| fade_level(f, recoveries, now_ms))
        .collect()
}

/// 整体和解分数 0..=100
pub fn growth_score(fragments: &[Fragment], recoveries: &[Recovery], now_ms: i64) -> f64 {
    if fragments.is_empty() {
        return 0.0;
    }
    let mut weighted_clarity = 0.0_f64;
    let mut weight_sum = 0.0_f64;
    for f in fragments {
        let clarity = fade_level(f, recoveries, now_ms);
        let w = f64::from(f.intensity.value());
        weighted_clarity += clarity * w;
        weight_sum += w;
    }
    if weight_sum == 0.0 {
        return 0.0;
    }
    let avg = weighted_clarity / weight_sum;
    ((1.0 - avg) * 100.0).clamp(0.0, 100.0)
}

/// 在 [start_ms, end_ms] 等距采样 `samples + 1` 个时间点的 growth_score。
///
/// - `samples` 被夹至 ≥1，避免除零。
/// - 如果 `end_ms <= start_ms`，返回 `samples + 1` 份同一个时点的 score。
/// - 这是 UI 描绘成长曲线的唯一权威入口，避免 Dart 写重复逻辑。
pub fn growth_score_series(
    fragments: &[Fragment],
    recoveries: &[Recovery],
    start_ms: i64,
    end_ms: i64,
    samples: u32,
) -> Vec<f64> {
    let n = samples.max(1) as i64;
    let span = (end_ms - start_ms).max(0);
    (0..=n)
        .map(|i| {
            let t = start_ms + (span * i / n);
            growth_score(fragments, recoveries, t)
        })
        .collect()
}

fn days_between(later_ms: i64, earlier_ms: i64) -> f64 {
    let diff = (later_ms - earlier_ms) as f64;
    diff / (1000.0 * 86_400.0)
}

#[cfg(test)]
mod tests {
    use super::super::Stage;
    use super::*;
    use crate::domain::{Fragment, Recovery};

    fn day_ms(days: i64) -> i64 {
        days * 86_400 * 1000
    }

    fn fragment(id: &str, created_day: i64, intensity: u8) -> Fragment {
        Fragment::try_new(id, day_ms(created_day), intensity, 270, Stage::Outburst)
            .expect("valid fragment")
    }

    fn recovery(id: &str, day: i64, intensity: u8, related: &[&str]) -> Recovery {
        Recovery::try_new(
            id,
            day_ms(day),
            intensity,
            "ok",
            related.iter().map(|s| (*s).to_string()).collect(),
        )
        .expect("valid recovery")
    }

    #[test]
    fn fresh_fragment_is_clear() {
        let f = fragment("a", 100, 3);
        assert!((fade_level(&f, &[], day_ms(100)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn old_fragment_fades_out() {
        let f = fragment("a", 0, 3);
        assert!(fade_level(&f, &[], day_ms(400)) < 0.05);
    }

    #[test]
    fn recovery_brightens_clarity_down() {
        let f = fragment("a", 0, 3);
        let now = day_ms(30);
        let baseline = fade_level(&f, &[], now);
        let r = recovery("r", 28, 5, &["a"]);
        let after = fade_level(&f, std::slice::from_ref(&r), now);
        assert!(after < baseline);
    }

    #[test]
    fn growth_score_grows_with_recovery() {
        let f = fragment("a", 0, 5);
        let now = day_ms(20);
        let before = growth_score(std::slice::from_ref(&f), &[], now);
        let r = recovery("r", 18, 5, &["a"]);
        let after = growth_score(&[f], std::slice::from_ref(&r), now);
        assert!(after > before);
    }

    #[test]
    fn empty_growth_score_is_zero() {
        assert!(growth_score(&[], &[], 0).abs() < 1e-9);
    }

    #[test]
    fn apply_fade_preserves_order() {
        let f1 = fragment("a", 0, 3);
        let f2 = fragment("b", 50, 3);
        let r = apply_fade(&[f1, f2], &[], day_ms(100));
        assert_eq!(r.len(), 2);
        assert!(r[1] > r[0]);
    }

    #[test]
    fn growth_score_series_length_is_samples_plus_one() {
        let f = fragment("a", 0, 3);
        let s = growth_score_series(std::slice::from_ref(&f), &[], day_ms(0), day_ms(30), 10);
        assert_eq!(s.len(), 11);
    }

    #[test]
    fn growth_score_series_endpoints_match_growth_score() {
        let f = fragment("a", 0, 3);
        let slice = std::slice::from_ref(&f);
        let series = growth_score_series(slice, &[], day_ms(10), day_ms(40), 8);
        let head = growth_score(slice, &[], day_ms(10));
        let tail = growth_score(slice, &[], day_ms(40));
        assert!((series.first().copied().unwrap() - head).abs() < 1e-9);
        assert!((series.last().copied().unwrap() - tail).abs() < 1e-9);
    }

    #[test]
    fn growth_score_series_clamps_samples_to_at_least_one() {
        // samples = 0 不能除零；应当返回 2 个点（i=0..=1）。
        let f = fragment("a", 0, 3);
        let s = growth_score_series(std::slice::from_ref(&f), &[], 0, day_ms(10), 0);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn growth_score_series_handles_inverted_range() {
        // end < start 时 span 被夹为 0，所有采样点位于 start 。
        let f = fragment("a", 0, 3);
        let s = growth_score_series(std::slice::from_ref(&f), &[], day_ms(20), day_ms(10), 4);
        assert_eq!(s.len(), 5);
        let first = s[0];
        assert!(s.iter().all(|x| (x - first).abs() < 1e-9));
    }
}
