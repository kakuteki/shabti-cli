use std::sync::{LazyLock, Mutex};

use fastembed::EmbeddingModel;
use fastembed_poc::{cosine_similarity, embed_texts, load_model};

// Shared model instances (Mutex needed because embed requires &mut self).
static ALL_MINI_LM: LazyLock<Mutex<fastembed::TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(load_model(EmbeddingModel::AllMiniLML6V2).expect("AllMiniLM model should load"))
});

static MULTILINGUAL_E5: LazyLock<Mutex<fastembed::TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(
        load_model(EmbeddingModel::MultilingualE5Small)
            .expect("MultilingualE5Small model should load"),
    )
});

static BGE_SMALL: LazyLock<Mutex<fastembed::TextEmbedding>> = LazyLock::new(|| {
    Mutex::new(load_model(EmbeddingModel::BGESmallENV15).expect("BGESmallEN model should load"))
});

// ── 0-4b: Text → Embedding generation ───────────────────────────────────

#[test]
fn all_minilm_loads_and_produces_correct_dimension() {
    let mut model = ALL_MINI_LM.lock().unwrap();
    let embeddings =
        embed_texts(&mut model, &["Hello, world!"]).expect("embedding should succeed");
    assert_eq!(embeddings.len(), 1);
    assert_eq!(
        embeddings[0].len(),
        384,
        "AllMiniLM-L6-v2 should produce 384-dim embeddings"
    );
}

#[test]
fn multilingual_e5_loads_and_produces_correct_dimension() {
    let mut model = MULTILINGUAL_E5.lock().unwrap();
    let embeddings =
        embed_texts(&mut model, &["query: Hello, world!"]).expect("embedding should succeed");
    assert_eq!(embeddings.len(), 1);
    assert_eq!(
        embeddings[0].len(),
        384,
        "multilingual-e5-small should produce 384-dim embeddings"
    );
}

#[test]
fn bge_small_loads_and_produces_correct_dimension() {
    let mut model = BGE_SMALL.lock().unwrap();
    let embeddings =
        embed_texts(&mut model, &["Hello, world!"]).expect("embedding should succeed");
    assert_eq!(embeddings.len(), 1);
    assert_eq!(
        embeddings[0].len(),
        384,
        "bge-small-en-v1.5 should produce 384-dim embeddings"
    );
}

// ── 0-4c: Model comparison ──────────────────────────────────────────────

#[test]
fn all_minilm_distinguishes_similar_from_different_texts() {
    let mut model = ALL_MINI_LM.lock().unwrap();
    let texts = &[
        "The cat sat on the mat",
        "A cat is sitting on a mat",
        "The stock market crashed today",
    ];
    let emb = embed_texts(&mut model, texts).expect("embedding should succeed");
    let sim_close = cosine_similarity(&emb[0], &emb[1]);
    let sim_far = cosine_similarity(&emb[0], &emb[2]);

    assert!(
        sim_close > sim_far,
        "similar texts ({sim_close:.4}) should score higher than different texts ({sim_far:.4})"
    );
    assert!(
        sim_close > 0.5,
        "similar texts should have reasonable similarity, got {sim_close:.4}"
    );
}

#[test]
fn bge_small_distinguishes_similar_from_different_texts() {
    let mut model = BGE_SMALL.lock().unwrap();
    let texts = &[
        "The cat sat on the mat",
        "A cat is sitting on a mat",
        "The stock market crashed today",
    ];
    let emb = embed_texts(&mut model, texts).expect("embedding should succeed");
    let sim_close = cosine_similarity(&emb[0], &emb[1]);
    let sim_far = cosine_similarity(&emb[0], &emb[2]);

    assert!(
        sim_close > sim_far,
        "similar texts ({sim_close:.4}) should score higher than different texts ({sim_far:.4})"
    );
    assert!(
        sim_close > 0.5,
        "similar texts should have reasonable similarity, got {sim_close:.4}"
    );
}

// ── 0-4d: Japanese accuracy evaluation ──────────────────────────────────

#[test]
fn multilingual_e5_japanese_similar_vs_different() {
    let mut model = MULTILINGUAL_E5.lock().unwrap();
    let texts = &[
        "query: 猫がマットの上に座っている",
        "query: マットの上に座る猫",
        "query: 今日の株式市場は暴落した",
    ];
    let emb = embed_texts(&mut model, texts).expect("embedding should succeed");
    let sim_close = cosine_similarity(&emb[0], &emb[1]);
    let sim_far = cosine_similarity(&emb[0], &emb[2]);

    assert!(
        sim_close > sim_far,
        "similar Japanese texts ({sim_close:.4}) should score higher than different ({sim_far:.4})"
    );
    assert!(
        sim_close > 0.7,
        "similar Japanese texts should have high similarity, got {sim_close:.4}"
    );
}

#[test]
fn multilingual_e5_cross_lingual_similarity() {
    let mut model = MULTILINGUAL_E5.lock().unwrap();
    let texts = &[
        "query: The cat is sitting on the mat",
        "query: 猫がマットの上に座っている",
        "query: The stock market crashed today",
    ];
    let emb = embed_texts(&mut model, texts).expect("embedding should succeed");
    let sim_cross = cosine_similarity(&emb[0], &emb[1]);
    let sim_diff = cosine_similarity(&emb[0], &emb[2]);

    assert!(
        sim_cross > sim_diff,
        "cross-lingual similar ({sim_cross:.4}) should beat different-topic ({sim_diff:.4})"
    );
    assert!(
        sim_cross > 0.5,
        "cross-lingual similarity should be meaningful, got {sim_cross:.4}"
    );
}

#[test]
fn multilingual_e5_japanese_multi_topic_discrimination() {
    let mut model = MULTILINGUAL_E5.lock().unwrap();
    let topics = &[
        "query: プログラミング言語Rustはメモリ安全性を保証する",
        "query: Rustは所有権システムでメモリ安全を実現している",
        "query: 東京タワーは1958年に完成した観光名所である",
        "query: 今日の天気は晴れで気温は25度です",
    ];
    let emb = embed_texts(&mut model, topics).expect("embedding should succeed");

    let sim_rust = cosine_similarity(&emb[0], &emb[1]);
    let sim_rust_vs_tower = cosine_similarity(&emb[0], &emb[2]);
    let sim_rust_vs_weather = cosine_similarity(&emb[0], &emb[3]);

    assert!(
        sim_rust > sim_rust_vs_tower,
        "same-topic ({sim_rust:.4}) should beat different-topic ({sim_rust_vs_tower:.4})"
    );
    assert!(
        sim_rust > sim_rust_vs_weather,
        "same-topic ({sim_rust:.4}) should beat different-topic ({sim_rust_vs_weather:.4})"
    );
}

// ── 0-4b/e: Batch embedding consistency and speed ───────────────────────

#[test]
fn batch_embedding_matches_single_embedding() {
    let mut model = ALL_MINI_LM.lock().unwrap();
    let text = "Hello, world!";
    let single = embed_texts(&mut model, &[text]).expect("single embed should work");
    let batch = embed_texts(&mut model, &[text, text]).expect("batch embed should work");

    let sim = cosine_similarity(&single[0], &batch[0]);
    assert!(
        (sim - 1.0).abs() < 1e-5,
        "single and batch embeddings should be identical, got similarity {sim}"
    );
}

#[test]
fn embedding_speed_100_texts() {
    let mut model = ALL_MINI_LM.lock().unwrap();
    let texts: Vec<&str> = (0..100)
        .map(|i| match i % 5 {
            0 => "The quick brown fox jumps over the lazy dog",
            1 => "Machine learning is a subset of artificial intelligence",
            2 => "Rust is a systems programming language",
            3 => "Tokyo is the capital of Japan",
            _ => "The weather is nice today",
        })
        .collect();

    let start = std::time::Instant::now();
    let embeddings = embed_texts(&mut model, &texts).expect("batch embed should work");
    let elapsed = start.elapsed();

    assert_eq!(embeddings.len(), 100);

    let per_embedding_ms = elapsed.as_millis() as f64 / 100.0;
    assert!(
        per_embedding_ms < 50.0,
        "embedding generation too slow: {per_embedding_ms:.1}ms per embedding (total {elapsed:?})"
    );

    eprintln!(
        "[perf] AllMiniLM 100 texts: {:.1}ms/embedding, {:.0}ms total",
        per_embedding_ms,
        elapsed.as_millis()
    );
}

#[test]
fn embedding_speed_1000_texts() {
    let mut model = ALL_MINI_LM.lock().unwrap();
    let texts: Vec<&str> = (0..1000)
        .map(|i| match i % 5 {
            0 => "The quick brown fox jumps over the lazy dog",
            1 => "Machine learning is a subset of artificial intelligence",
            2 => "Rust is a systems programming language",
            3 => "Tokyo is the capital of Japan",
            _ => "The weather is nice today",
        })
        .collect();

    let start = std::time::Instant::now();
    let embeddings = embed_texts(&mut model, &texts).expect("batch embed should work");
    let elapsed = start.elapsed();

    assert_eq!(embeddings.len(), 1000);

    let per_embedding_ms = elapsed.as_millis() as f64 / 1000.0;
    assert!(
        per_embedding_ms < 50.0,
        "embedding generation too slow: {per_embedding_ms:.1}ms per embedding (total {elapsed:?})"
    );

    eprintln!(
        "[perf] AllMiniLM 1000 texts: {:.1}ms/embedding, {:.0}ms total",
        per_embedding_ms,
        elapsed.as_millis()
    );
}
