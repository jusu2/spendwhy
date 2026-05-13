//! 淡化与和解分数算法。
//!
//! 纯函数：相同输入相同输出，不依赖时间外部源。`now_ms` 由调用方传入。

use super::{Fragment, Recovery};

/// 单条碎片清晰度 0..=1（1 = 清晰，0 = 几乎不可见）
pub fn fade_level(fragment: &Fragment, recoveries: &[Recovery], now_ms: i64) -> f64 {
    let age_days = days_between(now_ms, fragment.created_at_ms);
    let time_factor = (age_days / fragment.fade_period_days as f64).clamp(0.0, 1.0);

    let mut recovery_factor = 0.0_f64;
    for r in recoveries {
        if !r.related_fragment_ids.iter().any(|id| id == &fragment.id) {
            continue;
        }
        let since = days_between(now_ms, r.created_at_ms).max(0.0);
        let time_weight = (-since / 60.0).exp();
        recovery_factor += (r.intensity as f64 / 5.0) * 0.25 * time_weight;
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
        let w = f.intensity as f64;
        weighted_clarity += clarity * w;
        weight_sum += w;
    }
    if weight_sum == 0.0 {
        return 0.0;
    }
    let avg = weighted_clarity / weight_sum;
    ((1.0 - avg) * 100.0).clamp(0.0, 100.0)
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
        Fragment {
            id: id.into(),
            created_at_ms: day_ms(created_day),
            intensity,
            fade_period_days: 270,
            stage: Stage::Outburst,
        }
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
        let r = Recovery {
            id: "r".into(),
            created_at_ms: day_ms(28),
            intensity: 5,
            description: "ok".into(),
            related_fragment_ids: vec!["a".into()],
        };
        let after = fade_level(&f, &[r], now);
        assert!(after < baseline);
    }

    #[test]
    fn growth_score_grows_with_recovery() {
        let f = fragment("a", 0, 5);
        let now = day_ms(20);
        let before = growth_score(std::slice::from_ref(&f), &[], now);
        let r = Recovery {
            id: "r".into(),
            created_at_ms: day_ms(18),
            intensity: 5,
            description: "ok".into(),
            related_fragment_ids: vec!["a".into()],
        };
        let after = growth_score(&[f], &[r], now);
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
}
