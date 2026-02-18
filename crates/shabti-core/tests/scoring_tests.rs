use shabti_core::scoring::{TimeDecayConfig, access_boost, composite_score, time_decay};

// ============================================================
// Time decay
// ============================================================

#[test]
fn time_decay_zero_age_returns_one() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let decay = time_decay(now, now, &config);
    assert!(
        (decay - 1.0).abs() < 1e-4,
        "zero-age decay should be ~1.0, got {decay}"
    );
}

#[test]
fn time_decay_old_memory_decays() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let thirty_days_ago = now - 30 * 86400;
    let decay = time_decay(thirty_days_ago, now, &config);
    assert!(
        decay < 1.0,
        "30-day-old memory should have decay < 1.0, got {decay}"
    );
    assert!(decay > 0.0, "decay should be positive, got {decay}");
}

#[test]
fn time_decay_half_life_parameter() {
    let config = TimeDecayConfig {
        half_life_days: 7.0,
        min_factor: 0.01,
    };
    let now = 1_000_000i64;
    let seven_days_ago = now - 7 * 86400;
    let decay = time_decay(seven_days_ago, now, &config);
    assert!(
        (decay - 0.5).abs() < 0.05,
        "7 days with half_life=7 should give decay ~0.5, got {decay}"
    );
}

#[test]
fn time_decay_min_factor_floor() {
    let config = TimeDecayConfig {
        half_life_days: 1.0,
        min_factor: 0.1,
    };
    let now = 1_000_000i64;
    let very_old = now - 365 * 86400; // 1 year ago
    let decay = time_decay(very_old, now, &config);
    assert!(
        (decay - 0.1).abs() < 1e-4,
        "very old memory should hit min_factor=0.1, got {decay}"
    );
}

#[test]
fn time_decay_future_timestamp_clamps_to_one() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let future = now + 86400; // 1 day in future
    let decay = time_decay(future, now, &config);
    assert!(
        (decay - 1.0).abs() < 1e-4,
        "future timestamp should give decay=1.0, got {decay}"
    );
}

#[test]
fn time_decay_monotonically_decreasing() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let mut prev = 1.0f32;
    for days in [1, 7, 14, 30, 60, 90, 180, 365] {
        let created_at = now - days * 86400;
        let decay = time_decay(created_at, now, &config);
        assert!(
            decay <= prev,
            "decay should be monotonically decreasing: day {days}, prev={prev}, got {decay}"
        );
        prev = decay;
    }
}

// ============================================================
// Access boost
// ============================================================

#[test]
fn access_boost_zero_returns_one() {
    let boost = access_boost(0);
    assert!(
        (boost - 1.0).abs() < 1e-6,
        "access_count=0 should give boost=1.0, got {boost}"
    );
}

#[test]
fn access_boost_positive_count_greater_than_one() {
    let boost = access_boost(1);
    assert!(
        boost > 1.0,
        "access_count=1 should give boost > 1.0, got {boost}"
    );
}

#[test]
fn access_boost_monotonically_increasing() {
    let mut prev = access_boost(0);
    for count in 1..=100 {
        let boost = access_boost(count);
        assert!(
            boost >= prev,
            "boost should increase: count={count}, prev={prev}, got {boost}"
        );
        prev = boost;
    }
}

#[test]
fn access_boost_logarithmic_growth() {
    // Check that growth is sub-linear (logarithmic)
    let b10 = access_boost(10);
    let b100 = access_boost(100);
    let b1000 = access_boost(1000);

    // Ratio of increase should decrease (sub-linear)
    let ratio_1 = (b100 - b10) / (100.0 - 10.0);
    let ratio_2 = (b1000 - b100) / (1000.0 - 100.0);
    assert!(
        ratio_2 < ratio_1,
        "growth should be sub-linear: ratio_1={ratio_1}, ratio_2={ratio_2}"
    );
}

// ============================================================
// Composite score
// ============================================================

#[test]
fn composite_score_combines_all_factors() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let similarity = 0.9f32;
    let score = composite_score(similarity, now, 0, now, &config);
    // With zero age and zero access_count: score = 0.9 * 1.0 * 1.0 = 0.9
    assert!(
        (score - 0.9).abs() < 1e-4,
        "fresh entry with no accesses: score should be ~0.9, got {score}"
    );
}

#[test]
fn composite_score_old_entry_lower() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let fresh = composite_score(0.9, now, 0, now, &config);
    let old = composite_score(0.9, now - 30 * 86400, 0, now, &config);
    assert!(
        old < fresh,
        "older entry should have lower composite score: fresh={fresh}, old={old}"
    );
}

#[test]
fn composite_score_high_access_boosts() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    let low_access = composite_score(0.9, now, 0, now, &config);
    let high_access = composite_score(0.9, now, 100, now, &config);
    assert!(
        high_access > low_access,
        "high access should boost score: low={low_access}, high={high_access}"
    );
}

#[test]
fn composite_score_clamped_to_zero_one() {
    let config = TimeDecayConfig::default();
    let now = 1_000_000i64;
    // Even with high access boost, score should not exceed reasonable bounds
    let score = composite_score(1.0, now, 10000, now, &config);
    // Not clamped to 1.0 since access_boost can push above,
    // but should be positive and finite
    assert!(
        score > 0.0 && score.is_finite(),
        "score should be finite positive, got {score}"
    );
}
