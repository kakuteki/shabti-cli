//! Small-scale F1 benchmark for event segmentation (2-10).
//!
//! 10 manually annotated Japanese conversations with ground truth boundaries.
//! Tests multiple (window_size, sensitivity) parameter combinations and
//! reports micro-averaged precision, recall, F1.
//!
//! Run: cargo test -p shabti-engine --test f1_benchmark -- --nocapture

use shabti_core::event::{
    AdaptiveThreshold, consecutive_distances, detect_boundaries_adaptive,
};
use shabti_core::traits::EmbeddingModel;
use shabti_embedding::FastEmbedModel;

struct TestConversation {
    name: &'static str,
    utterances: &'static [&'static str],
    /// Ground truth boundary indices in the distances array.
    /// A boundary at index i means a topic change between utterance[i] and utterance[i+1].
    ground_truth: &'static [usize],
}

fn test_conversations() -> Vec<TestConversation> {
    vec![
        // 1. 料理→天気 (boundary between utterance 2 and 3)
        TestConversation {
            name: "料理→天気",
            utterances: &[
                "今日は鶏肉の照り焼きを作ろうと思います",
                "醤油とみりんを同量で混ぜるのがコツです",
                "仕上げにゴマを振りかけると美味しくなります",
                "ところで明日の天気予報を見ましたか",
                "台風が近づいているらしく大雨になるそうです",
                "洗濯物を早めに取り込んだほうがいいですね",
            ],
            ground_truth: &[2],
        },
        // 2. プログラミング→音楽→スポーツ (2 boundaries)
        TestConversation {
            name: "プログラミング→音楽→スポーツ",
            utterances: &[
                "Rustの所有権システムはメモリ安全性を保証します",
                "コンパイラがライフタイムを厳密にチェックします",
                "非同期処理にはtokioランタイムがよく使われます",
                "最近ジャズピアノの練習を始めました",
                "ビル・エヴァンスのアルバムを毎日聴いています",
                "週末はサッカーの試合を観に行きました",
                "日本代表のワールドカップ予選が楽しみです",
            ],
            ground_truth: &[2, 4],
        },
        // 3. 旅行→仕事 (boundary between utterance 2 and 3)
        TestConversation {
            name: "旅行→仕事",
            utterances: &[
                "京都の紅葉は11月が一番見頃です",
                "嵐山の竹林はとても幻想的でした",
                "金閣寺のライトアップも素晴らしかったです",
                "来週から新しいプロジェクトが始まります",
                "チームメンバーは5人でアジャイル開発を進めます",
                "デッドラインは3月末の予定です",
            ],
            ground_truth: &[2],
        },
        // 4. ペット→健康・運動 (boundary between utterance 2 and 3)
        TestConversation {
            name: "ペット→健康",
            utterances: &[
                "うちの猫は毎朝6時に起こしに来ます",
                "キャットタワーの上が一番のお気に入り場所です",
                "猫じゃらしで遊ぶと走り回って可愛いです",
                "最近ジョギングを始めて体調がよくなりました",
                "毎日30分走るだけで睡眠の質が改善されます",
                "食事も野菜中心に変えようと思っています",
            ],
            ground_truth: &[2],
        },
        // 5. 映画→料理 (boundary between utterance 2 and 3)
        TestConversation {
            name: "映画→料理",
            utterances: &[
                "先週の新作映画はアクションシーンが迫力満点でした",
                "最近のCG技術の進歩には驚かされます",
                "映画館の音響システムも素晴らしかったです",
                "週末にパスタを一から手作りしてみました",
                "トマトソースは生のトマトから煮込むと格別です",
                "次はカルボナーラに挑戦したいと思います",
            ],
            ground_truth: &[2],
        },
        // 6. 歴史→テクノロジー (boundary between utterance 2 and 3)
        TestConversation {
            name: "歴史→テクノロジー",
            utterances: &[
                "戦国時代の武将の中では織田信長が好きです",
                "桶狭間の戦いは日本史上最も劇的な戦いです",
                "関ヶ原の合戦で天下の行方が決まりました",
                "最近のAI技術の発展は目覚ましいです",
                "大規模言語モデルが様々な分野で活用されています",
                "自動運転技術も実用化が近づいています",
            ],
            ground_truth: &[2],
        },
        // 7. 宇宙（単一トピック — 境界なし）
        TestConversation {
            name: "宇宙（単一トピック）",
            utterances: &[
                "国際宇宙ステーションは地球を約90分で一周します",
                "火星探査ローバーが新しい岩石サンプルを採取しました",
                "ジェームズ・ウェッブ望遠鏡の画像は息をのむ美しさです",
                "ブラックホールの直接撮影は天文学の革命でした",
                "将来的に月面基地の建設計画が進んでいます",
                "宇宙デブリの問題も深刻化しています",
            ],
            ground_truth: &[],
        },
        // 8. 買い物→運動→読書 (2 boundaries, short segments)
        TestConversation {
            name: "買い物→運動→読書",
            utterances: &[
                "新しいスマートフォンを買おうか迷っています",
                "セールで洋服をたくさん買ってしまいました",
                "毎朝ヨガをするようになって体が柔らかくなりました",
                "水泳は全身運動なのでダイエットに効果的です",
                "最近村上春樹の新作を読み終えました",
                "図書館で毎週3冊は借りるようにしています",
            ],
            ground_truth: &[1, 3],
        },
        // 9. 科学→芸術 (boundary between utterance 2 and 3)
        TestConversation {
            name: "科学→芸術",
            utterances: &[
                "量子コンピュータは従来のコンピュータとは全く異なる原理で動作します",
                "遺伝子編集技術CRISPRが医療分野に革命をもたらしています",
                "素粒子物理学の標準模型を超える理論が求められています",
                "印象派の画家モネの睡蓮シリーズは世界中で愛されています",
                "現代アートは観る者の解釈に委ねる作品が多いです",
                "美術館巡りが週末の楽しみになっています",
            ],
            ground_truth: &[2],
        },
        // 10. 教育→自然→食べ物 (2 boundaries)
        TestConversation {
            name: "教育→自然→食べ物",
            utterances: &[
                "オンライン学習プラットフォームが急速に普及しています",
                "プログラミング教育が小学校でも必修化されました",
                "英語は早期教育が効果的だと言われています",
                "桜の開花予想が発表されて春が待ち遠しいです",
                "紅葉の名所を巡るのが秋の楽しみです",
                "お寿司の中ではサーモンが一番好きです",
                "ラーメンは味噌味が一番美味しいと思います",
            ],
            ground_truth: &[2, 4],
        },
    ]
}

/// Compute micro-averaged precision, recall, F1 over all conversations.
/// Uses tolerance window for matching (detected boundary at index i matches
/// ground truth at index j if |i - j| <= tolerance).
fn micro_averaged_f1(
    all_detected: &[Vec<usize>],
    all_ground_truth: &[&[usize]],
    tolerance: usize,
) -> (f32, f32, f32) {
    let mut total_tp: u32 = 0;
    let mut total_fp: u32 = 0;
    let mut total_fn: u32 = 0;

    for (detected, ground_truth) in all_detected.iter().zip(all_ground_truth.iter()) {
        let mut matched_gt = vec![false; ground_truth.len()];
        let mut tp: u32 = 0;
        let mut fp: u32 = 0;

        for &d_idx in detected {
            let mut found = false;
            for (gi, &gt_idx) in ground_truth.iter().enumerate() {
                if !matched_gt[gi] && d_idx.abs_diff(gt_idx) <= tolerance {
                    tp += 1;
                    matched_gt[gi] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                fp += 1;
            }
        }

        let fn_count = ground_truth.len() as u32 - tp;
        total_tp += tp;
        total_fp += fp;
        total_fn += fn_count;
    }

    let precision = if total_tp + total_fp > 0 {
        total_tp as f32 / (total_tp + total_fp) as f32
    } else {
        1.0 // no detections, no false positives
    };
    let recall = if total_tp + total_fn > 0 {
        total_tp as f32 / (total_tp + total_fn) as f32
    } else {
        1.0 // no ground truth boundaries
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    (precision, recall, f1)
}

#[test]
fn f1_benchmark_adaptive_threshold() {
    let model = FastEmbedModel::new().expect("failed to load embedding model");
    let conversations = test_conversations();

    // Pre-embed all utterances
    let all_embeddings: Vec<Vec<Vec<f32>>> = conversations
        .iter()
        .map(|conv| {
            conv.utterances
                .iter()
                .map(|u| model.embed(u).expect("embedding failed"))
                .collect()
        })
        .collect();

    // Pre-compute distances
    let all_distances: Vec<Vec<f32>> = all_embeddings
        .iter()
        .map(|embs| consecutive_distances(embs))
        .collect();

    // Ground truth references
    let all_ground_truth: Vec<&[usize]> = conversations
        .iter()
        .map(|c| c.ground_truth)
        .collect();

    // Parameter grid
    let window_sizes = [2, 3, 5];
    let sensitivities = [1.0f32, 1.5, 2.0, 2.5];
    let tolerance = 1; // allow off-by-one

    println!("\n{:=<70}", "");
    println!("F1 Benchmark: 10 conversations, 15 ground truth boundaries");
    println!("Tolerance: {tolerance}");
    println!("{:=<70}", "");

    let mut best_f1 = 0.0f32;
    let mut best_params = (0usize, 0.0f32);

    for &ws in &window_sizes {
        for &sens in &sensitivities {
            let config = AdaptiveThreshold {
                window_size: ws,
                sensitivity: sens,
            };

            let all_detected: Vec<Vec<usize>> = all_distances
                .iter()
                .map(|dists| {
                    detect_boundaries_adaptive(dists, &config)
                        .iter()
                        .map(|b| b.index)
                        .collect()
                })
                .collect();

            let (precision, recall, f1) =
                micro_averaged_f1(&all_detected, &all_ground_truth, tolerance);

            println!(
                "  w={ws} s={sens:.1}  P={precision:.3}  R={recall:.3}  F1={f1:.3}"
            );

            if f1 > best_f1 {
                best_f1 = f1;
                best_params = (ws, sens);
            }
        }
    }

    println!("{:=<70}", "");
    println!(
        "Best: window_size={}, sensitivity={:.1} → F1={:.3}",
        best_params.0, best_params.1, best_f1
    );
    println!("{:=<70}", "");

    // Per-conversation detail with best config
    let best_config = AdaptiveThreshold {
        window_size: best_params.0,
        sensitivity: best_params.1,
    };
    println!("\nPer-conversation detail (best config):");
    for (i, conv) in conversations.iter().enumerate() {
        let detected: Vec<usize> = detect_boundaries_adaptive(&all_distances[i], &best_config)
            .iter()
            .map(|b| b.index)
            .collect();
        let gt: Vec<usize> = conv.ground_truth.to_vec();
        let status = if detected_matches_ground_truth(&detected, &gt, tolerance) {
            "OK"
        } else {
            "MISS"
        };
        println!(
            "  [{status:>4}] {:<25} GT={gt:?}  Det={detected:?}",
            conv.name
        );
    }

    assert!(
        best_f1 >= 0.75,
        "Best F1 ({best_f1:.3}) should be >= 0.75"
    );
}

/// Check if detected boundaries match ground truth within tolerance.
fn detected_matches_ground_truth(
    detected: &[usize],
    ground_truth: &[usize],
    tolerance: usize,
) -> bool {
    if detected.len() != ground_truth.len() {
        return false;
    }
    let mut matched = vec![false; ground_truth.len()];
    for &d in detected {
        for (gi, &gt_idx) in ground_truth.iter().enumerate() {
            if !matched[gi] && d.abs_diff(gt_idx) <= tolerance {
                matched[gi] = true;
                break;
            }
        }
    }
    matched.iter().all(|&m| m)
}

/// Sanity check: distances within a topic should be smaller than across topics.
#[test]
fn distance_pattern_sanity_check() {
    let model = FastEmbedModel::new().expect("failed to load embedding model");

    // Embed a simple 2-topic conversation
    let utterances = [
        "猫は可愛いペットです",
        "犬も素晴らしい動物です",
        "株式市場は今日大幅に下落しました",
        "経済指標が悪化しています",
    ];

    let embeddings: Vec<Vec<f32>> = utterances
        .iter()
        .map(|u| model.embed(u).unwrap())
        .collect();

    let distances = consecutive_distances(&embeddings);

    // Distance between topic clusters should be larger
    assert!(
        distances[1] > distances[0],
        "cross-topic distance ({:.4}) should be > within-topic ({:.4})",
        distances[1],
        distances[0]
    );
    assert!(
        distances[1] > distances[2],
        "cross-topic distance ({:.4}) should be > within-topic ({:.4})",
        distances[1],
        distances[2]
    );
}
