use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use hdbscan_poc::{cluster_default, normalize_vectors, summarize_clusters};

fn load_model() -> TextEmbedding {
    TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::MultilingualE5Small)
            .with_show_download_progress(true),
    )
    .expect("model should load")
}

fn embed(model: &mut TextEmbedding, texts: &[&str]) -> Vec<Vec<f32>> {
    model.embed(texts.to_vec(), None).expect("embedding should succeed")
}

/// Multi-topic text samples for clustering evaluation.
/// 3 clear topics with 5 texts each.
fn topic_texts() -> Vec<&'static str> {
    vec![
        // Topic 0: Programming / Rust (5 texts)
        "query: Rustの所有権システムについて解説します",
        "query: borrowチェッカーはコンパイル時にメモリ安全性を検証する",
        "query: ライフタイムはRustの参照の有効期間を表す",
        "query: cargoはRustのパッケージマネージャーでありビルドツールです",
        "query: Rustのトレイトはインターフェースのような型システムの機能です",
        // Topic 1: Cooking / Food (5 texts)
        "query: 味噌ラーメンの作り方を紹介します",
        "query: だしは昆布と鰹節で取るのが基本です",
        "query: カレーライスは日本の国民食として愛されています",
        "query: 寿司は新鮮な魚介類を酢飯の上にのせた料理です",
        "query: 天ぷらは食材を衣で包んで揚げる調理法です",
        // Topic 2: Space / Astronomy (5 texts)
        "query: 火星は地球の隣に位置する赤い惑星です",
        "query: ブラックホールは光さえも脱出できない天体です",
        "query: 国際宇宙ステーションは地球の軌道を周回している",
        "query: 太陽系には8つの惑星が存在しています",
        "query: 天の川銀河には数千億の恒星が含まれている",
    ]
}

#[test]
fn real_embeddings_form_meaningful_clusters() {
    let mut model = load_model();
    let texts = topic_texts();

    let embeddings = embed(&mut model, &texts);
    let normed = normalize_vectors(&embeddings);

    let labels = cluster_default(&normed, 3).expect("clustering should succeed");
    let summary = summarize_clusters(&labels);

    eprintln!("labels: {:?}", labels);
    eprintln!("summary: {:?}", summary);

    // With 3 distinct topics, HDBSCAN should find at least 2 clusters
    assert!(
        summary.num_clusters >= 2,
        "should find at least 2 clusters from 3 topics, got {}: {:?}",
        summary.num_clusters, summary
    );
}

#[test]
fn cluster_labels_group_same_topic_together() {
    let mut model = load_model();
    let texts = topic_texts();

    let embeddings = embed(&mut model, &texts);
    let normed = normalize_vectors(&embeddings);

    let labels = cluster_default(&normed, 3).expect("clustering should succeed");

    eprintln!("labels: {:?}", labels);

    // Check that texts within the same topic tend to share labels.
    // For each topic group (indices 0-4, 5-9, 10-14), count the majority label.
    let groups = [&labels[0..5], &labels[5..10], &labels[10..15]];
    let mut correctly_grouped = 0;

    for group in &groups {
        let non_noise: Vec<i32> = group.iter().copied().filter(|&l| l >= 0).collect();
        if non_noise.is_empty() {
            continue;
        }
        // Count most frequent label in this group
        let mut counts = std::collections::HashMap::new();
        for &l in &non_noise {
            *counts.entry(l).or_insert(0) += 1;
        }
        let max_count = counts.values().max().unwrap();
        // If majority (>= 3 out of 5) share a label, consider it correctly grouped
        if *max_count >= 3 {
            correctly_grouped += 1;
        }
    }

    assert!(
        correctly_grouped >= 2,
        "at least 2 of 3 topic groups should have majority labels, got {correctly_grouped}"
    );
}

#[test]
fn min_cluster_size_tuning() {
    let mut model = load_model();
    let texts = topic_texts();

    let embeddings = embed(&mut model, &texts);
    let normed = normalize_vectors(&embeddings);

    for min_size in [2, 3, 4, 5] {
        let labels = cluster_default(&normed, min_size).expect("clustering should succeed");
        let summary = summarize_clusters(&labels);
        eprintln!(
            "min_cluster_size={min_size}: clusters={}, noise={}, sizes={:?}",
            summary.num_clusters, summary.noise_count, summary.cluster_sizes
        );
    }

    // With min_cluster_size=3, should find at least some clusters
    let labels_3 = cluster_default(&normed, 3).expect("clustering should succeed");
    let summary_3 = summarize_clusters(&labels_3);
    assert!(
        summary_3.num_clusters >= 1,
        "min_cluster_size=3 should find at least 1 cluster: {:?}",
        summary_3
    );
}
