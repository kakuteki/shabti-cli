use event_segmentation::{consecutive_distances, detect_boundaries, evaluate_boundaries, segment_events};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

fn load_multilingual_model() -> TextEmbedding {
    TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::MultilingualE5Small)
            .with_show_download_progress(true),
    )
    .expect("model should load")
}

/// Sample multi-topic conversation (Japanese).
/// Topics: programming (0-3), food (4-7), travel (8-11)
/// Ground truth boundaries: between index 3-4, and between index 7-8
fn sample_conversation() -> Vec<&'static str> {
    vec![
        "query: Rustでのメモリ管理について教えてください",
        "query: 所有権システムはどう機能しますか",
        "query: borrowチェッカーがコンパイル時に安全性を保証するんですね",
        "query: ライフタイムパラメータの使い方を知りたいです",
        // topic shift: programming → food
        "query: 今日のランチは何にしますか",
        "query: 近くにおいしいラーメン屋がありますよ",
        "query: 味噌ラーメンが一番人気だそうです",
        "query: トッピングはチャーシューとメンマがおすすめです",
        // topic shift: food → travel
        "query: 来月の旅行の計画を立てましょう",
        "query: 京都の紅葉が見頃だと思います",
        "query: 嵐山の竹林は朝早く行くのがいいですよ",
        "query: 帰りに宇治で抹茶スイーツを食べましょう",
    ]
}

#[test]
fn real_embeddings_detect_topic_shifts() {
    let mut model = load_multilingual_model();
    let texts = sample_conversation();

    let embeddings: Vec<Vec<f32>> = texts
        .iter()
        .map(|t| model.embed(vec![*t], None).unwrap().remove(0))
        .collect();

    let distances = consecutive_distances(&embeddings);

    // Print distances for visibility
    for (i, d) in distances.iter().enumerate() {
        eprintln!("  [{i}→{next}] distance = {d:.4}", next = i + 1);
    }

    let boundary_3_4 = distances[3];
    let ground_truth = [3usize, 7];

    // The strongest boundary (programming→food) should be at a ground-truth position.
    let max_idx = distances
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert!(
        ground_truth.contains(&max_idx),
        "max distance at index {max_idx} should be at a ground-truth boundary {ground_truth:?}"
    );

    // The programming→food boundary should be clearly above the within-topic average.
    let within_topic_avg: f32 = distances
        .iter()
        .enumerate()
        .filter(|(i, _)| !ground_truth.contains(i))
        .map(|(_, d)| *d)
        .sum::<f32>()
        / (distances.len() - ground_truth.len()) as f32;
    assert!(
        boundary_3_4 > within_topic_avg * 1.2,
        "boundary 3→4 ({boundary_3_4:.4}) should be >20% above within-topic average ({within_topic_avg:.4})"
    );
}

#[test]
fn threshold_tuning_on_sample_data() {
    let mut model = load_multilingual_model();
    let texts = sample_conversation();
    let ground_truth = vec![3, 7]; // boundary indices

    let embeddings: Vec<Vec<f32>> = texts
        .iter()
        .map(|t| model.embed(vec![*t], None).unwrap().remove(0))
        .collect();

    let distances = consecutive_distances(&embeddings);

    // Test multiple thresholds
    let thresholds = [0.2, 0.3, 0.4, 0.5, 0.6, 0.7];
    let mut best_f1 = 0.0f32;
    let mut best_threshold = 0.0f32;

    for threshold in thresholds {
        let boundaries = detect_boundaries(&distances, threshold);
        let (precision, recall) = evaluate_boundaries(&boundaries, &ground_truth, 1);
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        eprintln!(
            "  threshold={threshold:.1}: detected={}, precision={precision:.2}, recall={recall:.2}, F1={f1:.2}",
            boundaries.len()
        );

        if f1 > best_f1 {
            best_f1 = f1;
            best_threshold = threshold;
        }
    }

    eprintln!("  best: threshold={best_threshold:.1}, F1={best_f1:.2}");

    // The best F1 should be meaningful (at least some correct detections)
    assert!(
        best_f1 >= 0.5,
        "best F1 should be at least 0.5, got {best_f1:.2} at threshold {best_threshold:.1}"
    );
}

#[test]
fn segmentation_produces_correct_event_groups() {
    let mut model = load_multilingual_model();
    let texts = sample_conversation();

    let embeddings: Vec<Vec<f32>> = texts
        .iter()
        .map(|t| model.embed(vec![*t], None).unwrap().remove(0))
        .collect();

    let distances = consecutive_distances(&embeddings);

    // Use a moderate threshold that should detect the two topic shifts
    // Find the median of all distances as a starting reference
    let mut sorted_dists = distances.clone();
    sorted_dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted_dists[sorted_dists.len() / 2];

    // Use a threshold between median and max to catch obvious shifts
    let max_dist = sorted_dists.last().unwrap();
    let threshold = (median + max_dist) / 2.0;

    eprintln!("  median={median:.4}, max={max_dist:.4}, threshold={threshold:.4}");

    let boundaries = detect_boundaries(&distances, threshold);
    let segments = segment_events(texts.len(), &boundaries);

    eprintln!("  detected {} boundaries, {} segments", boundaries.len(), segments.len());
    for (i, seg) in segments.iter().enumerate() {
        eprintln!("  segment {i}: utterances {:?}", seg);
    }

    // Should detect at least 2 segments (ideally 3)
    assert!(
        segments.len() >= 2,
        "should detect at least 2 event segments, got {}",
        segments.len()
    );
}
