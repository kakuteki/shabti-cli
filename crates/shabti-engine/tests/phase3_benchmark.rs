use shabti_core::query::QueryBuilder;
use shabti_engine::{EngineConfig, ShabtiEngine, StoreOptions};
use tempfile::TempDir;
use uuid::Uuid;

fn test_config(dir: &TempDir) -> EngineConfig {
    let collection = format!("bench_{}", Uuid::new_v4().to_string().replace('-', ""));
    EngineConfig {
        qdrant_url: "http://localhost:6334".to_string(),
        collection_name: collection,
        data_dir: dir.path().to_path_buf(),
        vector_size: 384,
    }
}

/// Corpus of 30 diverse English sentences for benchmarking.
const BENCHMARK_CORPUS: &[&str] = &[
    "Cats are wonderful companions that bring joy",
    "Dogs are loyal and faithful friends to humans",
    "The stock market experienced significant volatility today",
    "Machine learning algorithms improve with more data",
    "Neural networks are inspired by biological brains",
    "Tokyo is the capital of Japan with many districts",
    "Climate change affects global weather patterns significantly",
    "Rust programming language ensures memory safety without garbage collection",
    "Quantum computing promises exponential speedup for certain problems",
    "The human genome contains approximately 3 billion base pairs",
    "Electric vehicles are becoming more affordable each year",
    "Blockchain technology enables decentralized trustless transactions",
    "Photosynthesis converts sunlight into chemical energy in plants",
    "Artificial intelligence transforms how we interact with technology",
    "The Renaissance period brought revolutionary changes to art and science",
    "Mediterranean diet is associated with improved cardiovascular health",
    "Space exploration has expanded our understanding of the universe",
    "Software engineering practices continue to evolve rapidly",
    "The Amazon rainforest is crucial for global oxygen production",
    "Cybersecurity threats are increasingly sophisticated and complex",
    "3D printing enables rapid prototyping in manufacturing",
    "The internet of things connects billions of devices worldwide",
    "Sustainable energy sources reduce dependence on fossil fuels",
    "Cognitive behavioral therapy is effective for anxiety disorders",
    "Ocean currents play a vital role in climate regulation",
    "Augmented reality merges digital content with physical environments",
    "The theory of relativity changed our understanding of spacetime",
    "Microbiome research reveals the importance of gut bacteria",
    "Cloud computing provides scalable on-demand computing resources",
    "Ancient civilizations developed sophisticated mathematical systems",
];

// ============================================================
// Recall@10: top-10 results should contain semantically related content
// ============================================================

#[tokio::test]
async fn recall_at_10() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    // Insert all corpus entries
    for &text in BENCHMARK_CORPUS {
        engine.store(text, &StoreOptions::default()).await.unwrap();
    }

    // Define query-relevance pairs: (query, expected substring in at least one top-10 result)
    let test_cases: Vec<(&str, Vec<&str>)> = vec![
        ("I love kittens and puppies", vec!["Cats", "Dogs"]),
        (
            "deep learning and AI",
            vec!["Neural", "Machine learning", "Artificial intelligence"],
        ),
        ("Rust memory safety", vec!["Rust programming"]),
        (
            "renewable energy and sustainability",
            vec!["Sustainable", "Electric vehicles"],
        ),
        (
            "biology and genetics",
            vec!["genome", "Photosynthesis", "Microbiome"],
        ),
    ];

    let mut hits = 0;
    let mut total = 0;

    for (query_text, expected_substrings) in &test_cases {
        let query = QueryBuilder::new(query_text).limit(10).build();
        let results = engine.execute_query(&query).await.unwrap();

        let contents: Vec<&str> = results.iter().map(|r| r.entry.content.as_str()).collect();

        for &expected in expected_substrings {
            total += 1;
            if contents.iter().any(|c| c.contains(expected)) {
                hits += 1;
            }
        }
    }

    let recall = hits as f64 / total as f64;
    println!("\n{:=<70}", "");
    println!("  Recall@10 Benchmark");
    println!("{:=<70}", "");
    println!("  Hits: {hits}/{total}");
    println!("  Recall@10: {recall:.3}");
    println!("{:=<70}\n", "");

    assert!(
        recall >= 0.85,
        "Recall@10 should be >= 0.85, got {recall:.3}"
    );

    engine.cleanup().await.unwrap();
}

// ============================================================
// Insert latency p99
// ============================================================

#[tokio::test]
async fn insert_latency_p99() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let mut latencies_ms = Vec::new();

    for &text in BENCHMARK_CORPUS {
        let start = std::time::Instant::now();
        engine.store(text, &StoreOptions::default()).await.unwrap();
        let elapsed = start.elapsed();
        latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
    }

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50_idx = (latencies_ms.len() as f64 * 0.50) as usize;
    let p99_idx = (latencies_ms.len() as f64 * 0.99) as usize;
    let p50 = latencies_ms[p50_idx.min(latencies_ms.len() - 1)];
    let p99 = latencies_ms[p99_idx.min(latencies_ms.len() - 1)];
    let mean = latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64;

    println!("\n{:=<70}", "");
    println!(
        "  Insert Latency Benchmark ({} entries)",
        BENCHMARK_CORPUS.len()
    );
    println!("{:=<70}", "");
    println!("  Mean:  {mean:.2} ms");
    println!("  p50:   {p50:.2} ms");
    println!("  p99:   {p99:.2} ms");
    println!("{:=<70}\n", "");

    // Target: p99 <= 100ms (includes embedding computation + Qdrant round-trip)
    // Note: ~50ms is embedding time, remainder is Qdrant insert
    assert!(
        p99 <= 100.0,
        "Insert p99 should be <= 100ms, got {p99:.2}ms"
    );

    engine.cleanup().await.unwrap();
}

// ============================================================
// Search latency p99
// ============================================================

#[tokio::test]
async fn search_latency_p99() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    // Insert corpus
    for &text in BENCHMARK_CORPUS {
        engine.store(text, &StoreOptions::default()).await.unwrap();
    }

    let queries = [
        "cats and dogs",
        "artificial intelligence",
        "climate change",
        "programming languages",
        "medical research",
        "space exploration",
        "energy sources",
        "ancient history",
        "computer security",
        "ocean biology",
    ];

    let mut latencies_ms = Vec::new();

    for &q in &queries {
        let query = QueryBuilder::new(q).limit(10).build();
        let start = std::time::Instant::now();
        let _results = engine.execute_query(&query).await.unwrap();
        let elapsed = start.elapsed();
        latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
    }

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50_idx = (latencies_ms.len() as f64 * 0.50) as usize;
    let p99_idx = (latencies_ms.len() as f64 * 0.99) as usize;
    let p50 = latencies_ms[p50_idx.min(latencies_ms.len() - 1)];
    let p99 = latencies_ms[p99_idx.min(latencies_ms.len() - 1)];
    let mean = latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64;

    println!("\n{:=<70}", "");
    println!(
        "  Search Latency Benchmark ({} queries over {} entries)",
        queries.len(),
        BENCHMARK_CORPUS.len()
    );
    println!("{:=<70}", "");
    println!("  Mean:  {mean:.2} ms");
    println!("  p50:   {p50:.2} ms");
    println!("  p99:   {p99:.2} ms");
    println!("{:=<70}\n", "");

    // Target: p99 <= 100ms (relaxed — 30 entries, includes embedding time)
    assert!(
        p99 <= 100.0,
        "Search p99 should be <= 100ms, got {p99:.2}ms"
    );

    engine.cleanup().await.unwrap();
}

// ============================================================
// Query with explanation generates valid scores
// ============================================================

#[tokio::test]
async fn benchmark_explanation_scores() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    for &text in &BENCHMARK_CORPUS[..10] {
        engine.store(text, &StoreOptions::default()).await.unwrap();
    }

    let query = QueryBuilder::new("animals and pets")
        .with_explanation()
        .limit(10)
        .build();
    let results = engine.execute_query(&query).await.unwrap();

    println!("\n{:=<70}", "");
    println!("  Score Explanation Breakdown");
    println!("{:=<70}", "");

    for r in &results {
        let exp = r.explanation.as_ref().unwrap();
        println!(
            "  [{:.4}] sim={:.4} decay={:.4} boost={:.4} | {}",
            r.score,
            exp.semantic_similarity,
            exp.time_decay_factor,
            exp.access_boost_factor,
            &r.entry.content[..r.entry.content.len().min(50)]
        );

        // Verify score consistency
        let computed = exp.final_score();
        assert!(
            (r.score - computed).abs() < 1e-4,
            "score mismatch: result={} computed={}",
            r.score,
            computed
        );
    }

    println!("{:=<70}\n", "");

    engine.cleanup().await.unwrap();
}
