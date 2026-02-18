/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Compute cosine distances between consecutive embedding pairs.
/// Distance = 1.0 - similarity. Returns a vector of length `embeddings.len() - 1`.
pub fn consecutive_distances(embeddings: &[Vec<f32>]) -> Vec<f32> {
    embeddings
        .windows(2)
        .map(|pair| 1.0 - cosine_similarity(&pair[0], &pair[1]))
        .collect()
}

/// A detected event boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct Boundary {
    /// Index position between utterances (boundary between [index] and [index+1]).
    pub index: usize,
    /// Cosine distance at this position.
    pub distance: f32,
}

/// Detect event boundaries where cosine distance strictly exceeds a threshold.
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

/// Segment utterances into events based on the detected boundaries.
/// Returns a vector of event groups, each containing utterance indices.
pub fn segment_events(num_utterances: usize, boundaries: &[Boundary]) -> Vec<Vec<usize>> {
    if num_utterances == 0 {
        return vec![];
    }
    let mut segments = Vec::new();
    let mut start = 0;
    for b in boundaries {
        segments.push((start..=b.index).collect());
        start = b.index + 1;
    }
    if start < num_utterances {
        segments.push((start..num_utterances).collect());
    }
    segments
}

/// Compute precision and recall of detected boundaries against ground truth.
/// A detected boundary matches ground truth if it's within `tolerance` positions.
/// Returns (0.0, 0.0) when either set is empty.
pub fn evaluate_boundaries(
    detected: &[Boundary],
    ground_truth: &[usize],
    tolerance: usize,
) -> (f32, f32) {
    if detected.is_empty() || ground_truth.is_empty() {
        return (0.0, 0.0);
    }

    let mut matched_gt: Vec<bool> = vec![false; ground_truth.len()];
    let mut true_positives = 0u32;

    for det in detected {
        for (gi, &gt_idx) in ground_truth.iter().enumerate() {
            if !matched_gt[gi] && det.index.abs_diff(gt_idx) <= tolerance {
                matched_gt[gi] = true;
                true_positives += 1;
                break;
            }
        }
    }

    let precision = true_positives as f32 / detected.len() as f32;
    let recall = true_positives as f32 / ground_truth.len() as f32;
    (precision, recall)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── cosine_similarity (already proven in fastembed-poc, minimal check) ─

    #[test]
    fn cosine_similarity_sanity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    // ── consecutive_distances ────────────────────────────────────────────

    #[test]
    fn consecutive_distances_length() {
        let embeddings = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.0, 1.0],
        ];
        let dists = consecutive_distances(&embeddings);
        assert_eq!(dists.len(), 2, "should have N-1 distances");
    }

    #[test]
    fn consecutive_distances_identical_is_zero() {
        let v = vec![1.0, 2.0, 3.0];
        let embeddings = vec![v.clone(), v.clone(), v.clone()];
        let dists = consecutive_distances(&embeddings);
        for d in &dists {
            assert!(d.abs() < 1e-6, "identical embeddings should have distance 0, got {d}");
        }
    }

    #[test]
    fn consecutive_distances_orthogonal_is_one() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
        ];
        let dists = consecutive_distances(&embeddings);
        assert!(
            (dists[0] - 1.0).abs() < 1e-6,
            "orthogonal vectors should have distance 1.0, got {}",
            dists[0]
        );
    }

    #[test]
    fn consecutive_distances_detects_topic_shift() {
        // Simulate: 3 similar embeddings, then a sharp shift
        let topic_a = vec![1.0, 0.1, 0.0];
        let topic_a2 = vec![0.95, 0.15, 0.0];
        let topic_a3 = vec![0.9, 0.2, 0.0];
        let topic_b = vec![0.0, 0.1, 1.0];

        let embeddings = vec![topic_a, topic_a2, topic_a3, topic_b];
        let dists = consecutive_distances(&embeddings);

        // Distance between topic_a variants should be small
        assert!(dists[0] < 0.1, "within-topic distance should be small: {}", dists[0]);
        assert!(dists[1] < 0.1, "within-topic distance should be small: {}", dists[1]);
        // Distance at topic shift should be large
        assert!(dists[2] > 0.5, "topic shift distance should be large: {}", dists[2]);
    }

    // ── detect_boundaries ────────────────────────────────────────────────

    #[test]
    fn detect_boundaries_no_exceeding_threshold() {
        let distances = vec![0.1, 0.2, 0.15];
        let boundaries = detect_boundaries(&distances, 0.5);
        assert!(boundaries.is_empty(), "no boundaries should be detected");
    }

    #[test]
    fn detect_boundaries_finds_peaks() {
        let distances = vec![0.1, 0.8, 0.15, 0.9, 0.1];
        let boundaries = detect_boundaries(&distances, 0.5);
        assert_eq!(boundaries.len(), 2);
        assert_eq!(boundaries[0].index, 1);
        assert_eq!(boundaries[1].index, 3);
    }

    #[test]
    fn detect_boundaries_exact_threshold_excluded() {
        let distances = vec![0.5, 0.5, 0.51];
        let boundaries = detect_boundaries(&distances, 0.5);
        assert_eq!(boundaries.len(), 1, "exact threshold should not trigger boundary");
        assert_eq!(boundaries[0].index, 2);
    }

    // ── segment_events ───────────────────────────────────────────────────

    #[test]
    fn segment_events_no_boundaries() {
        let segments = segment_events(5, &[]);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0], vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn segment_events_single_boundary() {
        let boundaries = vec![Boundary { index: 2, distance: 0.8 }];
        let segments = segment_events(5, &boundaries);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], vec![0, 1, 2]);
        assert_eq!(segments[1], vec![3, 4]);
    }

    #[test]
    fn segment_events_multiple_boundaries() {
        let boundaries = vec![
            Boundary { index: 1, distance: 0.7 },
            Boundary { index: 3, distance: 0.9 },
        ];
        let segments = segment_events(6, &boundaries);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0], vec![0, 1]);
        assert_eq!(segments[1], vec![2, 3]);
        assert_eq!(segments[2], vec![4, 5]);
    }

    // ── evaluate_boundaries ──────────────────────────────────────────────

    #[test]
    fn evaluate_perfect_detection() {
        let detected = vec![
            Boundary { index: 2, distance: 0.8 },
            Boundary { index: 5, distance: 0.9 },
        ];
        let ground_truth = vec![2, 5];
        let (precision, recall) = evaluate_boundaries(&detected, &ground_truth, 0);
        assert!((precision - 1.0).abs() < 1e-6, "precision should be 1.0, got {precision}");
        assert!((recall - 1.0).abs() < 1e-6, "recall should be 1.0, got {recall}");
    }

    #[test]
    fn evaluate_with_false_positive() {
        let detected = vec![
            Boundary { index: 2, distance: 0.8 },
            Boundary { index: 4, distance: 0.6 },
            Boundary { index: 5, distance: 0.9 },
        ];
        let ground_truth = vec![2, 5];
        let (precision, recall) = evaluate_boundaries(&detected, &ground_truth, 0);
        assert!(
            (precision - 2.0 / 3.0).abs() < 1e-6,
            "precision should be 2/3, got {precision}"
        );
        assert!((recall - 1.0).abs() < 1e-6, "recall should be 1.0, got {recall}");
    }

    #[test]
    fn evaluate_with_missed_boundary() {
        let detected = vec![Boundary { index: 2, distance: 0.8 }];
        let ground_truth = vec![2, 5];
        let (precision, recall) = evaluate_boundaries(&detected, &ground_truth, 0);
        assert!((precision - 1.0).abs() < 1e-6, "precision should be 1.0, got {precision}");
        assert!((recall - 0.5).abs() < 1e-6, "recall should be 0.5, got {recall}");
    }

    #[test]
    fn evaluate_with_tolerance() {
        let detected = vec![Boundary { index: 3, distance: 0.8 }];
        let ground_truth = vec![2]; // off by 1
        let (precision, recall) = evaluate_boundaries(&detected, &ground_truth, 1);
        assert!((precision - 1.0).abs() < 1e-6, "within-tolerance should match");
        assert!((recall - 1.0).abs() < 1e-6, "within-tolerance should match");
    }

    #[test]
    fn evaluate_empty_detected() {
        let detected: Vec<Boundary> = vec![];
        let ground_truth = vec![2, 5];
        let (precision, recall) = evaluate_boundaries(&detected, &ground_truth, 0);
        // No detections: precision is undefined (0/0), we return 0.0
        assert_eq!(precision, 0.0);
        assert_eq!(recall, 0.0);
    }

    #[test]
    fn evaluate_empty_ground_truth() {
        let detected = vec![Boundary { index: 2, distance: 0.8 }];
        let ground_truth: Vec<usize> = vec![];
        let (precision, recall) = evaluate_boundaries(&detected, &ground_truth, 0);
        assert_eq!(precision, 0.0);
        // No ground truth: recall is undefined, we return 0.0
        assert_eq!(recall, 0.0);
    }
}
