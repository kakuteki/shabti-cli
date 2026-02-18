use shabti_core::gate::{DataTier, FeatureGate};

#[test]
fn tier_minimal_below_10() {
    assert_eq!(DataTier::from_count(0), DataTier::Minimal);
    assert_eq!(DataTier::from_count(1), DataTier::Minimal);
    assert_eq!(DataTier::from_count(9), DataTier::Minimal);
}

#[test]
fn tier_basic_10_to_99() {
    assert_eq!(DataTier::from_count(10), DataTier::Basic);
    assert_eq!(DataTier::from_count(50), DataTier::Basic);
    assert_eq!(DataTier::from_count(99), DataTier::Basic);
}

#[test]
fn tier_standard_100_to_999() {
    assert_eq!(DataTier::from_count(100), DataTier::Standard);
    assert_eq!(DataTier::from_count(500), DataTier::Standard);
    assert_eq!(DataTier::from_count(999), DataTier::Standard);
}

#[test]
fn tier_full_1000_plus() {
    assert_eq!(DataTier::from_count(1000), DataTier::Full);
    assert_eq!(DataTier::from_count(10000), DataTier::Full);
    assert_eq!(DataTier::from_count(100000), DataTier::Full);
}

#[test]
fn gate_minimal_features() {
    let gate = FeatureGate::from_count(5);
    assert_eq!(gate.tier, DataTier::Minimal);
    assert!(gate.flat_search);
    assert!(!gate.knn_links);
    assert!(!gate.hdbscan_clustering);
    assert!(!gate.multi_resolution);
    assert!(gate.time_decay);
}

#[test]
fn gate_basic_features() {
    let gate = FeatureGate::from_count(50);
    assert_eq!(gate.tier, DataTier::Basic);
    assert!(gate.flat_search);
    assert!(gate.knn_links);
    assert!(!gate.hdbscan_clustering);
    assert!(!gate.multi_resolution);
    assert!(gate.time_decay);
}

#[test]
fn gate_standard_features() {
    let gate = FeatureGate::from_count(500);
    assert_eq!(gate.tier, DataTier::Standard);
    assert!(gate.flat_search);
    assert!(gate.knn_links);
    assert!(gate.hdbscan_clustering);
    assert!(!gate.multi_resolution);
    assert!(gate.time_decay);
}

#[test]
fn gate_full_features() {
    let gate = FeatureGate::from_count(5000);
    assert_eq!(gate.tier, DataTier::Full);
    assert!(gate.flat_search);
    assert!(gate.knn_links);
    assert!(gate.hdbscan_clustering);
    assert!(gate.multi_resolution);
    assert!(gate.time_decay);
}

#[test]
fn gate_boundary_9_to_10() {
    let g9 = FeatureGate::from_count(9);
    let g10 = FeatureGate::from_count(10);
    assert_eq!(g9.tier, DataTier::Minimal);
    assert_eq!(g10.tier, DataTier::Basic);
    assert!(!g9.knn_links);
    assert!(g10.knn_links);
}

#[test]
fn gate_boundary_99_to_100() {
    let g99 = FeatureGate::from_count(99);
    let g100 = FeatureGate::from_count(100);
    assert_eq!(g99.tier, DataTier::Basic);
    assert_eq!(g100.tier, DataTier::Standard);
    assert!(!g99.hdbscan_clustering);
    assert!(g100.hdbscan_clustering);
}

#[test]
fn gate_boundary_999_to_1000() {
    let g999 = FeatureGate::from_count(999);
    let g1000 = FeatureGate::from_count(1000);
    assert_eq!(g999.tier, DataTier::Standard);
    assert_eq!(g1000.tier, DataTier::Full);
    assert!(!g999.multi_resolution);
    assert!(g1000.multi_resolution);
}
