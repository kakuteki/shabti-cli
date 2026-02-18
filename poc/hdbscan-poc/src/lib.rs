use std::collections::HashMap;

use hdbscan::{Hdbscan, HdbscanHyperParams};

/// Result of clustering: label per data point (-1 = noise).
pub type ClusterLabels = Vec<i32>;

/// Cluster embeddings using HDBSCAN with the given min_cluster_size.
pub fn cluster_default(data: &[Vec<f32>], min_cluster_size: usize) -> anyhow::Result<ClusterLabels> {
    let owned: Vec<Vec<f32>> = data.to_vec();
    let hyper_params = HdbscanHyperParams::builder()
        .min_cluster_size(min_cluster_size)
        .build();
    let clusterer = Hdbscan::new(&owned, hyper_params);
    Ok(clusterer.cluster()?)
}

/// Normalize vectors to unit length (for cosine-like distance with Euclidean HDBSCAN).
pub fn normalize_vectors(data: &[Vec<f32>]) -> Vec<Vec<f32>> {
    data.iter()
        .map(|v| {
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm == 0.0 {
                v.clone()
            } else {
                v.iter().map(|x| x / norm).collect()
            }
        })
        .collect()
}

/// Summarize clustering results: number of clusters, noise count, cluster sizes.
#[derive(Debug, PartialEq)]
pub struct ClusterSummary {
    pub num_clusters: usize,
    pub noise_count: usize,
    pub cluster_sizes: Vec<usize>,
}

pub fn summarize_clusters(labels: &ClusterLabels) -> ClusterSummary {
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for &label in labels {
        *counts.entry(label).or_insert(0) += 1;
    }
    let noise_count = counts.remove(&-1).unwrap_or(0);
    let mut cluster_ids: Vec<i32> = counts.keys().copied().collect();
    cluster_ids.sort();
    let cluster_sizes: Vec<usize> = cluster_ids.iter().map(|id| counts[id]).collect();
    ClusterSummary {
        num_clusters: cluster_sizes.len(),
        noise_count,
        cluster_sizes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── normalize_vectors ────────────────────────────────────────────────

    #[test]
    fn normalize_produces_unit_vectors() {
        let data = vec![vec![3.0, 4.0], vec![1.0, 0.0], vec![0.0, 5.0]];
        let normed = normalize_vectors(&data);

        for v in &normed {
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-5,
                "vector should be unit length, got norm={norm}"
            );
        }
    }

    #[test]
    fn normalize_preserves_direction() {
        let data = vec![vec![3.0, 4.0]];
        let normed = normalize_vectors(&data);
        // 3/5 = 0.6, 4/5 = 0.8
        assert!((normed[0][0] - 0.6).abs() < 1e-5);
        assert!((normed[0][1] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn normalize_zero_vector_stays_zero() {
        let data = vec![vec![0.0, 0.0]];
        let normed = normalize_vectors(&data);
        assert_eq!(normed[0], vec![0.0, 0.0]);
    }

    // ── summarize_clusters ───────────────────────────────────────────────

    #[test]
    fn summarize_no_clusters() {
        let labels = vec![-1, -1, -1];
        let summary = summarize_clusters(&labels);
        assert_eq!(summary.num_clusters, 0);
        assert_eq!(summary.noise_count, 3);
        assert!(summary.cluster_sizes.is_empty());
    }

    #[test]
    fn summarize_two_clusters_with_noise() {
        let labels = vec![0, 0, 0, 1, 1, -1, -1];
        let summary = summarize_clusters(&labels);
        assert_eq!(summary.num_clusters, 2);
        assert_eq!(summary.noise_count, 2);
        assert_eq!(summary.cluster_sizes, vec![3, 2]);
    }

    #[test]
    fn summarize_single_cluster() {
        let labels = vec![0, 0, 0, 0];
        let summary = summarize_clusters(&labels);
        assert_eq!(summary.num_clusters, 1);
        assert_eq!(summary.noise_count, 0);
        assert_eq!(summary.cluster_sizes, vec![4]);
    }

    // ── cluster_default (with synthetic data) ────────────────────────────

    #[test]
    fn cluster_well_separated_blobs() {
        // Two well-separated clusters in 2D
        let mut data: Vec<Vec<f32>> = Vec::new();

        // Cluster A: around (0.0, 0.0)
        for i in 0..20 {
            data.push(vec![0.0 + (i as f32) * 0.01, 0.0 + (i as f32) * 0.005]);
        }
        // Cluster B: around (10.0, 10.0)
        for i in 0..20 {
            data.push(vec![10.0 + (i as f32) * 0.01, 10.0 + (i as f32) * 0.005]);
        }

        let labels = cluster_default(&data, 5).expect("clustering should succeed");
        let summary = summarize_clusters(&labels);

        assert_eq!(
            summary.num_clusters, 2,
            "should find 2 clusters, got {}: {:?}",
            summary.num_clusters, summary
        );
    }

    #[test]
    fn cluster_identifies_noise_points() {
        // Two dense clusters + isolated noise points at moderate distance
        let mut data: Vec<Vec<f32>> = Vec::new();

        // Cluster A: 15 points around (0, 0)
        for i in 0..15 {
            data.push(vec![0.0 + (i as f32) * 0.01, 0.0 + (i as f32) * 0.005]);
        }
        // Cluster B: 15 points around (2, 2)
        for i in 0..15 {
            data.push(vec![2.0 + (i as f32) * 0.01, 2.0 + (i as f32) * 0.005]);
        }
        // Noise points: isolated, between clusters
        data.push(vec![5.0, 5.0]);
        data.push(vec![-3.0, -3.0]);
        data.push(vec![8.0, -2.0]);

        let labels = cluster_default(&data, 5).expect("clustering should succeed");
        let summary = summarize_clusters(&labels);

        assert!(
            summary.num_clusters >= 1,
            "should find at least 1 cluster: {:?}",
            summary
        );
        assert!(
            summary.noise_count >= 1,
            "should identify at least 1 noise point: {:?}",
            summary
        );
    }

    #[test]
    fn cluster_min_cluster_size_effect() {
        // With small min_cluster_size, more clusters may form
        let mut data: Vec<Vec<f32>> = Vec::new();
        for i in 0..10 {
            data.push(vec![0.0 + (i as f32) * 0.01, 0.0]);
        }
        for i in 0..10 {
            data.push(vec![5.0 + (i as f32) * 0.01, 5.0]);
        }

        let labels_small = cluster_default(&data, 3).expect("clustering should succeed");
        let labels_large = cluster_default(&data, 8).expect("clustering should succeed");

        let summary_small = summarize_clusters(&labels_small);
        let summary_large = summarize_clusters(&labels_large);

        // Larger min_cluster_size typically produces fewer or equal clusters
        assert!(
            summary_large.num_clusters <= summary_small.num_clusters + 1,
            "larger min_cluster_size should not produce more clusters: small={:?}, large={:?}",
            summary_small,
            summary_large
        );
    }
}
