use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An event groups related memory entries (e.g., a conversation topic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub entry_ids: Vec<Uuid>,
    pub start_time: i64,
    pub end_time: i64,
    pub summary_embedding: Option<Vec<f32>>,
}

impl Event {
    pub fn new(entry_ids: Vec<Uuid>, start_time: i64, end_time: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            entry_ids,
            start_time,
            end_time,
            summary_embedding: None,
        }
    }
}

/// Boundary detected between consecutive entries.
#[derive(Debug, Clone)]
pub struct Boundary {
    /// Index position: boundary is between [index] and [index+1].
    pub index: usize,
    /// Cosine distance at this boundary.
    pub distance: f32,
}

/// Configuration for adaptive threshold boundary detection.
#[derive(Debug, Clone)]
pub struct AdaptiveThreshold {
    /// Number of neighbors to consider for moving statistics.
    pub window_size: usize,
    /// Number of standard deviations above mean to trigger boundary.
    pub sensitivity: f32,
}

/// Compute cosine similarity between two vectors.
/// Returns 0.0 if either vector is zero.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Compute cosine distance (1 - similarity) between two vectors.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

/// Compute cosine distances between consecutive embedding pairs.
/// Returns N-1 distances for N embeddings.
pub fn consecutive_distances(embeddings: &[Vec<f32>]) -> Vec<f32> {
    embeddings
        .windows(2)
        .map(|pair| cosine_distance(&pair[0], &pair[1]))
        .collect()
}

/// Detect boundaries where distance strictly exceeds a fixed threshold.
pub fn detect_boundaries(distances: &[f32], threshold: f32) -> Vec<Boundary> {
    distances
        .iter()
        .enumerate()
        .filter(|(_, d)| **d > threshold)
        .map(|(i, d)| Boundary {
            index: i,
            distance: *d,
        })
        .collect()
}

/// Detect boundaries using adaptive threshold (moving average + stddev).
/// Falls back to a fixed threshold (median-based) when data is too small
/// for meaningful statistics.
pub fn detect_boundaries_adaptive(distances: &[f32], config: &AdaptiveThreshold) -> Vec<Boundary> {
    if distances.is_empty() {
        return Vec::new();
    }

    // Fixed threshold fallback for very small data (< 3 distances)
    const FALLBACK_THRESHOLD: f32 = 0.3;
    if distances.len() < 3 {
        return detect_boundaries(distances, FALLBACK_THRESHOLD);
    }

    // Compute global statistics for use as a floor
    let global_mean: f32 = distances.iter().sum::<f32>() / distances.len() as f32;
    let global_variance: f32 = distances
        .iter()
        .map(|d| (d - global_mean).powi(2))
        .sum::<f32>()
        / distances.len() as f32;
    let global_std = global_variance.sqrt();

    // Minimum std floor to prevent near-zero std from causing false positives
    const MIN_STD: f32 = 0.02;

    let half_window = config.window_size / 2;
    let mut boundaries = Vec::new();

    for i in 0..distances.len() {
        // Compute local statistics excluding the current point
        let start = i.saturating_sub(half_window);
        let end = (i + half_window + 1).min(distances.len());

        let neighbors: Vec<f32> = (start..end)
            .filter(|&j| j != i)
            .map(|j| distances[j])
            .collect();

        if neighbors.is_empty() {
            continue;
        }

        let local_mean: f32 = neighbors.iter().sum::<f32>() / neighbors.len() as f32;
        let local_variance: f32 = neighbors
            .iter()
            .map(|d| (d - local_mean).powi(2))
            .sum::<f32>()
            / neighbors.len() as f32;
        let local_std = local_variance.sqrt();

        // Use the maximum of local std, global std, and minimum floor
        let effective_std = local_std.max(global_std).max(MIN_STD);
        let threshold = local_mean + config.sensitivity * effective_std;

        if distances[i] > threshold {
            boundaries.push(Boundary {
                index: i,
                distance: distances[i],
            });
        }
    }

    boundaries
}

/// Split utterance indices into event segments based on boundaries.
/// Boundaries are assumed to be sorted by index.
pub fn segment_events(num_utterances: usize, boundaries: &[Boundary]) -> Vec<Vec<usize>> {
    if num_utterances == 0 {
        return Vec::new();
    }

    let boundary_indices: Vec<usize> = boundaries.iter().map(|b| b.index).collect();
    let mut segments = Vec::new();
    let mut start = 0;

    for &bi in &boundary_indices {
        // Segment includes entries from start through bi (inclusive)
        let end = bi + 1; // boundary is between [bi] and [bi+1]
        if start < end && end <= num_utterances {
            segments.push((start..end).collect());
            start = end;
        }
    }

    // Remaining entries after the last boundary
    if start < num_utterances {
        segments.push((start..num_utterances).collect());
    }

    segments
}

/// Detect time gaps where the interval exceeds the threshold.
/// Returns boundary indices (between [i] and [i+1]).
pub fn detect_time_gaps(timestamps: &[i64], gap_threshold_secs: i64) -> Vec<usize> {
    timestamps
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| (pair[1] - pair[0]).abs() >= gap_threshold_secs)
        .map(|(i, _)| i)
        .collect()
}

/// Merge semantic boundaries with temporal boundary indices.
/// Deduplicates by index, preferring semantic distance when available.
/// Returns sorted by index.
pub fn merge_boundaries(semantic: &[Boundary], temporal_indices: &[usize]) -> Vec<Boundary> {
    let mut by_index: HashMap<usize, f32> = HashMap::new();

    // Add semantic boundaries (they have real distance values)
    for b in semantic {
        by_index.insert(b.index, b.distance);
    }

    // Add temporal boundaries (only if not already present from semantic)
    for &idx in temporal_indices {
        by_index.entry(idx).or_insert(0.0); // 0.0 = temporal-only, no cosine distance
    }

    let mut merged: Vec<Boundary> = by_index
        .into_iter()
        .map(|(index, distance)| Boundary { index, distance })
        .collect();
    merged.sort_by_key(|b| b.index);
    merged
}

/// Evaluate detected boundaries against ground truth.
/// Returns (precision, recall). Uses tolerance window for matching.
/// Returns (0.0, 0.0) if either detected or ground_truth is empty.
pub fn evaluate_boundaries(
    detected: &[Boundary],
    ground_truth: &[usize],
    tolerance: usize,
) -> (f32, f32) {
    if detected.is_empty() || ground_truth.is_empty() {
        return (0.0, 0.0);
    }

    // Count true positives for precision: how many detected match a ground truth
    let mut matched_gt: Vec<bool> = vec![false; ground_truth.len()];
    let mut true_positives = 0u32;

    for d in detected {
        for (gi, &gt_idx) in ground_truth.iter().enumerate() {
            if !matched_gt[gi] && d.index.abs_diff(gt_idx) <= tolerance {
                true_positives += 1;
                matched_gt[gi] = true;
                break;
            }
        }
    }

    let precision = true_positives as f32 / detected.len() as f32;
    let recall = true_positives as f32 / ground_truth.len() as f32;
    (precision, recall)
}
