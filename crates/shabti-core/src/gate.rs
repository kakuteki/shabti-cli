use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataTier {
    Minimal,
    Basic,
    Standard,
    Full,
}

impl DataTier {
    pub fn from_count(count: usize) -> Self {
        match count {
            0..10 => DataTier::Minimal,
            10..100 => DataTier::Basic,
            100..1000 => DataTier::Standard,
            _ => DataTier::Full,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FeatureGate {
    pub tier: DataTier,
    pub flat_search: bool,
    pub knn_links: bool,
    pub hdbscan_clustering: bool,
    pub multi_resolution: bool,
    pub time_decay: bool,
}

impl FeatureGate {
    pub fn from_count(count: usize) -> Self {
        let tier = DataTier::from_count(count);
        match tier {
            DataTier::Minimal => Self {
                tier,
                flat_search: true,
                knn_links: false,
                hdbscan_clustering: false,
                multi_resolution: false,
                time_decay: true,
            },
            DataTier::Basic => Self {
                tier,
                flat_search: true,
                knn_links: true,
                hdbscan_clustering: false,
                multi_resolution: false,
                time_decay: true,
            },
            DataTier::Standard => Self {
                tier,
                flat_search: true,
                knn_links: true,
                hdbscan_clustering: true,
                multi_resolution: false,
                time_decay: true,
            },
            DataTier::Full => Self {
                tier,
                flat_search: true,
                knn_links: true,
                hdbscan_clustering: true,
                multi_resolution: true,
                time_decay: true,
            },
        }
    }

    pub fn tier(&self) -> DataTier {
        self.tier
    }
}
