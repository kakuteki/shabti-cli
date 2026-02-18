use qdrant_poc::*;

/// Generate a deterministic pseudo-random vector of the given dimension.
fn dummy_vector(seed: u64, dim: usize) -> Vec<f32> {
    let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (0..dim)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((state >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

/// Generate a vector similar to the seed vector (small perturbation).
fn similar_vector(base: &[f32], offset: f32) -> Vec<f32> {
    base.iter().map(|x| x + offset * 0.01).collect()
}

// ── Connection and collection setup ─────────────────────────────────────

#[tokio::test]
async fn client_connects_and_creates_collection() {
    let client = create_client(DEFAULT_URL).expect("client should connect");
    setup_collection(&client).await.expect("collection setup should succeed");
    let count = count_points(&client).await.expect("count should work");
    assert_eq!(count, 0, "new collection should be empty");
}

// ── Insert and count ────────────────────────────────────────────────────

#[tokio::test]
async fn insert_and_count_points() {
    let client = create_client(DEFAULT_URL).unwrap();
    setup_collection(&client).await.unwrap();

    let points: Vec<PointData> = (1..=100)
        .map(|i| PointData {
            id: i,
            vector: dummy_vector(i, VECTOR_DIM as usize),
            content: format!("memory entry {i}"),
            created_at: 1_700_000_000 + i as i64 * 3600,
        })
        .collect();

    insert_points(&client, points).await.expect("insert should succeed");

    let count = count_points(&client).await.unwrap();
    assert_eq!(count, 100, "should have 100 points after insert");
}

// ── HNSW similarity search ──────────────────────────────────────────────

#[tokio::test]
async fn search_returns_most_similar() {
    let client = create_client(DEFAULT_URL).unwrap();
    setup_collection(&client).await.unwrap();

    let base_vec = dummy_vector(42, VECTOR_DIM as usize);
    let similar_vec = similar_vector(&base_vec, 0.1);
    let different_vec = dummy_vector(999, VECTOR_DIM as usize);

    let points = vec![
        PointData {
            id: 1,
            vector: base_vec.clone(),
            content: "base".into(),
            created_at: 1_700_000_000,
        },
        PointData {
            id: 2,
            vector: similar_vec,
            content: "similar".into(),
            created_at: 1_700_001_000,
        },
        PointData {
            id: 3,
            vector: different_vec,
            content: "different".into(),
            created_at: 1_700_002_000,
        },
    ];

    insert_points(&client, points).await.unwrap();

    let results = search_similar(&client, base_vec, 3).await.unwrap();

    assert!(!results.is_empty(), "should return results");
    // Top result should be the base itself (exact match), then similar
    assert_eq!(results[0].id, 1, "top result should be the exact match");
    assert_eq!(results[1].id, 2, "second result should be the similar vector");
    assert!(
        results[0].score > results[1].score,
        "exact match score ({}) should be higher than similar ({})",
        results[0].score,
        results[1].score
    );
    assert!(
        results[1].score > results[2].score,
        "similar score ({}) should be higher than different ({})",
        results[1].score,
        results[2].score
    );
}

// ── Payload filter: time range ──────────────────────────────────────────

#[tokio::test]
async fn search_with_time_range_filter() {
    let client = create_client(DEFAULT_URL).unwrap();
    setup_collection(&client).await.unwrap();

    // Insert 10 points with timestamps spread across 10 hours
    let base_time = 1_700_000_000i64;
    let query_vec = dummy_vector(1, VECTOR_DIM as usize);

    let points: Vec<PointData> = (1..=10)
        .map(|i| PointData {
            id: i,
            vector: similar_vector(&query_vec, i as f32 * 0.5),
            content: format!("entry {i}"),
            created_at: base_time + i as i64 * 3600,
        })
        .collect();

    insert_points(&client, points).await.unwrap();

    // Search only within the first 5 hours
    let time_from = base_time;
    let time_to = base_time + 5 * 3600 + 1;

    let results = search_with_time_filter(
        &client,
        query_vec,
        10,
        Some(time_from),
        Some(time_to),
    )
    .await
    .unwrap();

    // All results should have created_at within the time range
    for r in &results {
        assert!(
            r.created_at >= time_from && r.created_at <= time_to,
            "result id={} created_at={} should be in range [{time_from}, {time_to}]",
            r.id,
            r.created_at
        );
    }
    assert!(
        results.len() <= 5,
        "at most 5 results should be in the first 5 hours, got {}",
        results.len()
    );
    assert!(
        !results.is_empty(),
        "should find some results in the time range"
    );
}

// ── Composite query: vector + time filter ───────────────────────────────

#[tokio::test]
async fn composite_query_vector_and_time() {
    let client = create_client(DEFAULT_URL).unwrap();
    setup_collection(&client).await.unwrap();

    let base_time = 1_700_000_000i64;
    let target_vec = dummy_vector(50, VECTOR_DIM as usize);

    // Insert: 5 recent + similar, 5 old + similar, 5 recent + different
    let mut points = Vec::new();
    for i in 0..5 {
        points.push(PointData {
            id: i + 1,
            vector: similar_vector(&target_vec, i as f32 * 0.1),
            content: format!("recent-similar-{i}"),
            created_at: base_time + 86400 * 10 + i as i64 * 100, // recent
        });
    }
    for i in 0..5 {
        points.push(PointData {
            id: i + 6,
            vector: similar_vector(&target_vec, i as f32 * 0.1),
            content: format!("old-similar-{i}"),
            created_at: base_time + i as i64 * 100, // old
        });
    }
    for i in 0..5 {
        points.push(PointData {
            id: i + 11,
            vector: dummy_vector(500 + i, VECTOR_DIM as usize),
            content: format!("recent-different-{i}"),
            created_at: base_time + 86400 * 10 + i as i64 * 100, // recent
        });
    }

    insert_points(&client, points).await.unwrap();

    // Search: similar to target AND recent only
    let results = search_with_time_filter(
        &client,
        target_vec,
        5,
        Some(base_time + 86400 * 5), // only recent
        None,
    )
    .await
    .unwrap();

    // Results should be recent-similar (not old-similar or recent-different)
    for r in &results {
        assert!(
            r.content.starts_with("recent-similar"),
            "result '{}' should be recent-similar, got id={} score={}",
            r.content,
            r.id,
            r.score
        );
    }
}

// ── Performance: bulk insert + search latency ───────────────────────────

#[tokio::test]
async fn performance_1k_insert_and_search() {
    let client = create_client(DEFAULT_URL).unwrap();
    setup_collection(&client).await.unwrap();

    // Insert 1000 points
    let points: Vec<PointData> = (1..=1000)
        .map(|i| PointData {
            id: i,
            vector: dummy_vector(i, VECTOR_DIM as usize),
            content: format!("entry {i}"),
            created_at: 1_700_000_000 + i as i64,
        })
        .collect();

    let insert_start = std::time::Instant::now();
    insert_points(&client, points).await.unwrap();
    let insert_elapsed = insert_start.elapsed();

    let count = count_points(&client).await.unwrap();
    assert_eq!(count, 1000);

    // Search 10 times and measure average
    let query = dummy_vector(500, VECTOR_DIM as usize);
    let search_start = std::time::Instant::now();
    for _ in 0..10 {
        let results = search_similar(&client, query.clone(), 10).await.unwrap();
        assert_eq!(results.len(), 10);
    }
    let search_elapsed = search_start.elapsed();
    let avg_search_ms = search_elapsed.as_millis() as f64 / 10.0;

    eprintln!(
        "[perf] 1K insert: {:?}, 10 searches: {:?} (avg {:.1}ms/search)",
        insert_elapsed, search_elapsed, avg_search_ms
    );

    assert!(
        avg_search_ms < 100.0,
        "average search should be under 100ms, got {avg_search_ms:.1}ms"
    );
}

#[tokio::test]
async fn performance_10k_insert_and_search() {
    let client = create_client(DEFAULT_URL).unwrap();
    setup_collection(&client).await.unwrap();

    let points: Vec<PointData> = (1..=10_000)
        .map(|i| PointData {
            id: i,
            vector: dummy_vector(i, VECTOR_DIM as usize),
            content: format!("entry {i}"),
            created_at: 1_700_000_000 + i as i64,
        })
        .collect();

    let insert_start = std::time::Instant::now();
    insert_points(&client, points).await.unwrap();
    let insert_elapsed = insert_start.elapsed();

    let count = count_points(&client).await.unwrap();
    assert_eq!(count, 10_000);

    let query = dummy_vector(5000, VECTOR_DIM as usize);
    let search_start = std::time::Instant::now();
    for _ in 0..10 {
        let results = search_similar(&client, query.clone(), 10).await.unwrap();
        assert_eq!(results.len(), 10);
    }
    let search_elapsed = search_start.elapsed();
    let avg_search_ms = search_elapsed.as_millis() as f64 / 10.0;

    eprintln!(
        "[perf] 10K insert: {:?}, 10 searches: {:?} (avg {:.1}ms/search)",
        insert_elapsed, search_elapsed, avg_search_ms
    );

    assert!(
        avg_search_ms < 200.0,
        "average search at 10K should be under 200ms, got {avg_search_ms:.1}ms"
    );
}
