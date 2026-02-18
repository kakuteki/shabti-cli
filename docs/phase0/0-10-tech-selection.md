# 0-10. 技術選定最終決定文書

Phase 0 の全 PoC 結果と研究を踏まえた技術スタック確定。

---

## PoC 結果サマリ

| PoC                    | ライブラリ/ツール    | 結果                                                                            | テスト数 | 判定     |
| ---------------------- | -------------------- | ------------------------------------------------------------------------------- | -------- | -------- |
| 0-3 Qdrant             | `qdrant-client` 1.16 | Docker モードで HNSW + payload filter + 複合クエリ動作確認。1K: <1ms, 10K: <5ms | 7        | **採用** |
| 0-4 fastembed-rs       | `fastembed` 5.9      | 3 モデル比較。384 次元。日本語精度良好。100 文 batch ~50ms                      | 17       | **採用** |
| 0-5 petgraph           | `petgraph`           | BFS/DFS 深さ制限走査 + JSON 永続化動作確認                                      | 13       | **採用** |
| 0-6 HDBSCAN            | `hdbscan` 0.12       | 密度ベースクラスタリング + ノイズ検出。実 embedding で意味のあるクラスタ形成    | 12       | **採用** |
| 0-7 napi-rs            | `napi-rs`            | 同期/非同期/構造体の Rust → Node.js ブリッジ動作確認                            | 14       | **採用** |
| 0-8 Event Segmentation | 自作                 | コサイン距離境界検出 + precision/recall 評価フレームワーク                      | 20       | **採用** |

**全 83 テストパス。全 PoC で致命的ブロッカーなし。**

---

## 技術スタック確定

### コアエンジン (Rust)

| コンポーネント       | 選定技術                    | 根拠                                                                           | 代替案                       | 不採用理由                                                |
| -------------------- | --------------------------- | ------------------------------------------------------------------------------ | ---------------------------- | --------------------------------------------------------- |
| **言語**             | Rust (edition 2024)         | メモリ安全性、ゼロコスト抽象化、FFI 容易性。PoC 全体で安定動作                 | Go, C++                      | Go: GC 停止が latency に影響。C++: メモリ安全性の保証なし |
| **Embedding**        | fastembed-rs (ONNX Runtime) | ローカル推論。ONNX ベースで GPU 不要。384 次元で十分な精度                     | candle, rust-bert            | candle: エコシステム未成熟。rust-bert: PyTorch 依存が重い |
| **Embedding モデル** | MultilingualE5Small         | 384 次元。日英両対応。PoC で日本語精度を確認                                   | AllMiniLML6V2, BGESmallENV15 | AllMiniLM: 英語特化。BGE: E5 と同等だが多言語対応が劣る   |
| **ベクトル DB**      | Qdrant (Docker / External)  | HNSW + payload filter + 複合クエリ。Rust クライアント成熟。1K/10K 性能確認済み | Milvus, Chroma               | Milvus: 重量級。Chroma: Python ネイティブ                 |
| **グラフ**           | petgraph                    | 純 Rust。BFS/DFS 走査 + serde 永続化。軽量                                     | neo4j, kuzu                  | neo4j: 外部サーバー必須。kuzu: C++ バインディングの安定性 |
| **クラスタリング**   | hdbscan 0.12                | 密度ベース。クラスタ数自動決定。ノイズ検出。実 embedding で有効性確認          | linfa-clustering             | linfa: HDBSCAN 未実装                                     |
| **Node.js ブリッジ** | napi-rs                     | 同期/非同期/構造体パス確認済み。TypeScript 型定義自動生成                      | wasm-bindgen                 | wasm: Qdrant/ONNX の WASM 対応が未成熟                    |
| **非同期ランタイム** | tokio                       | デファクトスタンダード。Qdrant クライアントが tokio 前提                       | async-std                    | エコシステムの広さで tokio                                |

### イベント分割アルゴリズム

| 要素                    | 選定                      | 根拠                                                                          |
| ----------------------- | ------------------------- | ----------------------------------------------------------------------------- |
| **一次境界検出**        | コサイン距離 + 適応的閾値 | PoC (0-8) で動作確認。EM-LLM の知見から `T = mean(D) + gamma * std(D)` に改良 |
| **補助境界**            | 時間間隔 (gap) 閾値       | 長時間の沈黙は強制的に境界                                                    |
| **境界精緻化**          | Modularity 最適化 (将来)  | EM-LLM で最大の品質向上。中程度の実装コスト。Phase 2 で検討                   |
| **Surprise ベース分割** | 不採用                    | 自己回帰モデルの予測分布が必要。エンコーダ embedding では適用不可             |

### メモリスコアリング

GAM の Three-Factor Scoring を改良:

```
score = w_relevance * relevance + w_recency * recency + w_importance * importance
```

| 要素       | 計算方法                                    | デフォルト重み |
| ---------- | ------------------------------------------- | -------------- |
| Relevance  | Qdrant コサイン類似度                       | 3.0            |
| Recency    | `decay_factor^elapsed_hours` (真の時間減衰) | 0.5            |
| Importance | アクセス頻度 + 被参照回数 + テキスト特徴量  | 2.0            |

重みはクエリごとにオーバーライド可能 (Composable Query DSL)。

### Enriched Embedding (A-MEM から採用)

```
embedding = encode(concat(content, keywords, tags))
```

キーワード/タグは RAKE/YAKE でローカル抽出 (LLM 不要)。

### MemoryEntry 構造 (論文知見を反映)

```rust
struct MemoryEntry {
    // 基本
    id: Uuid,
    content: String,
    content_hash: [u8; 32],        // SHA-256
    embedding: Vec<f32>,
    model_id: String,

    // 時間
    created_at: i64,
    session_id: String,
    gap_before: f64,
    gap_after: f64,

    // イベント
    event_id: Option<String>,

    // メタデータ (A-MEM inspired)
    keywords: Vec<String>,          // RAKE/YAKE 抽出
    tags: Vec<String>,              // ルールベース分類

    // スコアリング (GAM inspired)
    access_count: u32,
    last_accessed: i64,

    // 重複・矛盾 (Mem0 inspired)
    dedupe_group_id: Option<String>,
    superseded_by: Option<Uuid>,

    // ライフサイクル (MemOS inspired)
    lifecycle_state: LifecycleState, // Active | Merged | Archived | Superseded
    origin_type: OriginType,         // UserInput | AgentGenerated | Imported

    // スコーピング
    namespace: String,
    metadata: HashMap<String, Value>,
}
```

---

## リスクと緩和策

| リスク                                   | 影響                                                   | 緩和策                                                        |
| ---------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------- |
| **Qdrant Embedded モードが Rust 未対応** | Docker 依存が残る。`npm install` だけでは動かない      | Phase 4 で Embedded 対応を再調査。代替: SQLite + 自前 HNSW    |
| **fastembed-rs のモデル切替コスト**      | embedding 再計算が必要                                 | model_id フィールドで管理。バックグラウンド再計算ジョブ       |
| **HDBSCAN のスケーラビリティ**           | 100K+ でメモリ/時間が問題になりうる                    | バックグラウンド再クラスタリング。サンプリング戦略            |
| **日本語 NLP の精度**                    | RAKE/YAKE のキーワード抽出が日本語で精度低下しうる     | 日本語対応トークナイザ (lindera) の検討。MeCab バインディング |
| **napi-rs クロスビルド**                 | win32/darwin/linux の 3 プラットフォームビルドの複雑さ | GitHub Actions matrix build。prebuild バイナリ配布            |

---

## Phase 1 開始判定

### ブロッカーチェック

| 項目                     | 状態   | 備考                                               |
| ------------------------ | ------ | -------------------------------------------------- |
| fastembed-rs 動作        | **OK** | 3 モデル、17 テストパス                            |
| Qdrant 接続・検索        | **OK** | 7 テストパス、1K/10K 性能確認                      |
| petgraph 走査・永続化    | **OK** | 13 テストパス                                      |
| HDBSCAN クラスタリング   | **OK** | 12 テストパス、実 embedding で有効性確認           |
| napi-rs ブリッジ         | **OK** | 14 テストパス、同期/非同期/構造体                  |
| イベント分割アルゴリズム | **OK** | 20 テストパス、precision/recall 評価フレームワーク |
| 論文精読                 | **OK** | 5 論文の知見を設計に反映                           |
| 競合調査                 | **OK** | 3 プロジェクトのデータモデル・API 比較完了         |
| ベンチマーク基準         | **OK** | 6 指標の目標値・計測方法確定                       |

### 判定

**全ブロッカー解消。Phase 1 (ストレージ + 生データアーカイブ) に進行可能。**

Phase 1 の最初のタスクは:

1. `1-9`: Cargo workspace 構造のセットアップ (crates: core, embedding, index, graph, storage, napi)
2. `1-1`: MemoryEntry データモデル定義 (上記構造体の確定)
3. `1-2`: Append-Only Log の実装

---

## 変更履歴

| 日付       | 変更内容                                                |
| ---------- | ------------------------------------------------------- |
| 2026-02-19 | 初版作成。全 PoC 結果 + 論文/競合調査を踏まえた技術選定 |
