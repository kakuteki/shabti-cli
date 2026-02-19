use shabti_core::metrics::EngineMetrics;

// --- 6-4b: Metrics collection ---

#[test]
fn metrics_initial_state() {
    let metrics = EngineMetrics::new();

    assert_eq!(metrics.store_count(), 0);
    assert_eq!(metrics.search_count(), 0);
    assert_eq!(metrics.error_count(), 0);
    assert_eq!(metrics.total_requests(), 0);
}

#[test]
fn metrics_increment_store() {
    let metrics = EngineMetrics::new();

    metrics.record_store();
    metrics.record_store();
    metrics.record_store();

    assert_eq!(metrics.store_count(), 3);
    assert_eq!(metrics.total_requests(), 3);
}

#[test]
fn metrics_increment_search() {
    let metrics = EngineMetrics::new();

    metrics.record_search(15);
    metrics.record_search(25);

    assert_eq!(metrics.search_count(), 2);
    assert_eq!(metrics.total_requests(), 2);
}

#[test]
fn metrics_increment_errors() {
    let metrics = EngineMetrics::new();

    metrics.record_error();
    metrics.record_error();

    assert_eq!(metrics.error_count(), 2);
}

#[test]
fn metrics_avg_search_latency() {
    let metrics = EngineMetrics::new();

    metrics.record_search(10);
    metrics.record_search(20);
    metrics.record_search(30);

    assert_eq!(metrics.avg_search_latency_ms(), 20);
}

#[test]
fn metrics_avg_search_latency_no_searches() {
    let metrics = EngineMetrics::new();
    assert_eq!(metrics.avg_search_latency_ms(), 0);
}

#[test]
fn metrics_thread_safe() {
    let metrics = std::sync::Arc::new(EngineMetrics::new());

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let m = metrics.clone();
            std::thread::spawn(move || {
                for _ in 0..100 {
                    m.record_store();
                    m.record_search(5);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(metrics.store_count(), 1000);
    assert_eq!(metrics.search_count(), 1000);
    assert_eq!(metrics.total_requests(), 2000);
}

#[test]
fn metrics_to_json() {
    let metrics = EngineMetrics::new();
    metrics.record_store();
    metrics.record_search(42);
    metrics.record_error();

    let json = metrics.to_json();
    assert_eq!(json["store_count"], 1);
    assert_eq!(json["search_count"], 1);
    assert_eq!(json["error_count"], 1);
    assert_eq!(json["avg_search_latency_ms"], 42);
    assert_eq!(json["total_requests"], 2);
}

#[test]
fn metrics_reset() {
    let metrics = EngineMetrics::new();
    metrics.record_store();
    metrics.record_search(10);
    metrics.record_error();

    metrics.reset();

    assert_eq!(metrics.store_count(), 0);
    assert_eq!(metrics.search_count(), 0);
    assert_eq!(metrics.error_count(), 0);
    assert_eq!(metrics.avg_search_latency_ms(), 0);
}
