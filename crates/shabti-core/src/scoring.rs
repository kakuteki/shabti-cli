/// Configuration for time decay scoring.
#[derive(Debug, Clone)]
pub struct TimeDecayConfig {
    /// Number of days for score to decay to 50%.
    pub half_life_days: f64,
    /// Minimum decay factor (floor).
    pub min_factor: f32,
}

impl Default for TimeDecayConfig {
    fn default() -> Self {
        Self {
            half_life_days: 30.0,
            min_factor: 0.05,
        }
    }
}

/// Compute time decay factor for a memory entry.
/// Returns a value in [min_factor, 1.0].
/// Uses exponential decay: exp(-lambda * age_days) where lambda = ln(2) / half_life_days.
pub fn time_decay(created_at: i64, now: i64, config: &TimeDecayConfig) -> f32 {
    let age_secs = (now - created_at).max(0) as f64;
    let age_days = age_secs / 86400.0;
    let lambda = (2.0f64).ln() / config.half_life_days;
    let decay = (-lambda * age_days).exp() as f32;
    decay.max(config.min_factor)
}

/// Compute access frequency boost.
/// Returns a value >= 1.0, growing logarithmically with access count.
/// Formula: 1.0 + log2(1.0 + access_count)
pub fn access_boost(access_count: u32) -> f32 {
    1.0 + (1.0 + access_count as f32).log2()
}

/// Compute composite score combining similarity, time decay, and access boost.
/// score = similarity * time_decay * access_boost
pub fn composite_score(
    similarity: f32,
    created_at: i64,
    access_count: u32,
    now: i64,
    config: &TimeDecayConfig,
) -> f32 {
    let decay = time_decay(created_at, now, config);
    let boost = access_boost(access_count);
    similarity * decay * boost
}
