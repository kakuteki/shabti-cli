# 0-9. ベンチマーク基準確定

Phase 0 研究タスク。評価データセットの選定、各指標の目標値と計測方法を文書化する。

---

## 評価データセット

### 主要データセット

| データセット    | 目的                                 | 理由                                                                             |
| --------------- | ------------------------------------ | -------------------------------------------------------------------------------- |
| **LoCoMo**      | 長期会話メモリ、temporal reasoning   | MemOS (1 位), Letta (74.0%), Mem0 (68.5%) のベンチマーク結果が存在し直接比較可能 |
| **LongMemEval** | 複数セッションにまたがる長期記憶評価 | MemOS がベンチマーク済み。セッション横断の記憶保持を評価                         |

### 補助データセット

| データセット               | 目的                       | 理由                                              |
| -------------------------- | -------------------------- | ------------------------------------------------- |
| **自作イベント分割データ** | イベント境界検出の F1 評価 | 100 会話 x 手動アノテーション。PoC (0-8) の拡張版 |
| **自作検索精度データ**     | Recall@K 評価              | 1K/10K 記憶に対する検索クエリ + 正解ラベル        |

### データセット準備手順

1. **LoCoMo**: 公開データセットを取得。shabti のデータモデルに変換するローダーを実装
2. **LongMemEval**: 同上
3. **イベント分割データ**: 日本語 + 英語の会話データ各 50 セッション。3-5 トピック/セッション。手動で境界をアノテーション
4. **検索精度データ**: 1K/10K 規模の記憶プール。100 クエリ x 正解記憶 ID のペアを作成

---

## 指標定義と目標値

### 1. Recall@10 (検索精度)

**定義**: クエリに対する正解記憶が上位 10 件に含まれる割合

```
Recall@10 = (上位 10 件に含まれる正解記憶数) / (全正解記憶数)
```

**目標値**: **>= 0.85**

**計測方法**:

1. 検索精度データセットから 100 クエリを実行
2. 各クエリの上位 10 件を取得
3. 正解ラベルとの一致率を計算
4. 全クエリの平均を算出

**比較対象**: Mem0 の検索精度 (vector-only) をベースラインとする

---

### 2. イベント分割 F1

**定義**: 自動検出されたイベント境界と手動アノテーション境界の一致度

```
Precision = (正しく検出された境界数) / (検出された全境界数)
Recall = (正しく検出された境界数) / (手動アノテーション全境界数)
F1 = 2 * Precision * Recall / (Precision + Recall)
```

**tolerance**: 前後 1 発話のずれを許容 (EM-LLM 論文の評価基準に準拠)

**目標値**: **F1 >= 0.75**

**計測方法**:

1. 100 会話のイベント分割データセットを用意
2. shabti のイベント分割アルゴリズムを適用
3. 手動アノテーションとの一致率 (tolerance=1) を計算
4. Precision/Recall/F1 を算出

**パラメータチューニング**: 適応的閾値の gamma 値を 0.5 / 1.0 / 1.5 / 2.0 で比較

---

### 3. 挿入遅延 p99

**定義**: 記憶挿入 (embedding 生成 + Qdrant 挿入 + メタデータ書き込み) の 99 パーセンタイル遅延

**目標値**: **p99 <= 5ms** (embedding 生成を除く)

**注**: embedding 生成 (fastembed-rs) は別途計測。AllMiniLML6V2 で ~1ms/文が PoC で確認済み。

**計測方法**:

1. 10K 件の記憶を順次挿入
2. 各挿入の所要時間を記録
3. p50, p95, p99 を算出
4. Qdrant Embedded モードと Docker モードの両方で計測

---

### 4. 検索遅延 p99

**定義**: 検索クエリ (embedding 生成 + Qdrant ANN + 後処理スコアリング + 再ランキング) の 99 パーセンタイル遅延

**目標値**: **p99 <= 20ms** (10K 記憶)

**計測方法**:

1. 10K 記憶が格納された状態で 1000 クエリを実行
2. 各クエリの所要時間を記録
3. p50, p95, p99 を算出
4. 単純 vector 検索、multi-resolution 検索、graph + vector 複合検索それぞれで計測

---

### 5. メモリ使用量 (RSS)

**定義**: プロセスの Resident Set Size

**目標値**: **Mem0 比 50% 以下** (同等データ量での比較)

**計測方法**:

1. 1K / 10K / 100K 記憶を格納
2. 各データ量での RSS を計測
3. 同一データを Mem0 (Python + Qdrant) に格納した場合の RSS と比較
4. Qdrant Embedded モードの追加メモリも含む

**計測ツール**: `/proc/[pid]/status` (Linux CI) / `tasklist` (Windows ローカル)

---

### 6. LoCoMo タスク精度 (Phase 3+ で計測)

**定義**: LoCoMo ベンチマークの各サブタスクにおける正答率

**サブタスク**:

- Single-hop QA
- Multi-hop QA
- Temporal reasoning
- Open-domain QA
- Adversarial QA

**目標値**: **Mem0 (68.5%) 以上** (検索エンジンとしての比較。LLM 推論部分は同一モデルを使用)

**注**: shabti は検索エンジンであり LLM ではないため、LoCoMo での評価は「shabti の検索結果を LLM に渡して回答させる」パイプラインで行う。shabti vs Mem0 vs Letta の検索品質比較となる。

---

## 計測手順書

### 環境

| 項目       | 仕様                                            |
| ---------- | ----------------------------------------------- |
| OS         | Ubuntu (GitHub Actions CI) / Windows (ローカル) |
| Rust       | stable (1.93+)                                  |
| Qdrant     | Embedded (Rust) / Docker (テスト環境)           |
| Embedding  | MultilingualE5Small (384 次元)                  |
| データ規模 | 1K / 10K / 100K                                 |

### 実行手順

```bash
# 1. ベンチマークバイナリのビルド
cargo build --release -p shabti-bench

# 2. 挿入性能テスト
shabti-bench insert --count 10000 --output results/insert.json

# 3. 検索性能テスト
shabti-bench search --queries queries.json --output results/search.json

# 4. イベント分割 F1 テスト
shabti-bench segment --data conversations.json --annotations annotations.json --output results/segment.json

# 5. メモリ使用量テスト
shabti-bench memory --count 10000 --output results/memory.json

# 6. 結果集約
shabti-bench report --input results/ --output benchmark-report.md
```

### CI 統合

Phase 3 完了後、`cargo bench` で自動実行。GitHub Actions で各 PR にベンチマーク結果をコメントとして投稿。回帰検出 (前回比 10% 以上劣化で警告)。

---

## 指標一覧

| #   | 指標            | 目標値           | 計測フェーズ | 比較対象             |
| --- | --------------- | ---------------- | ------------ | -------------------- |
| 1   | Recall@10       | >= 0.85          | Phase 3      | Mem0 vector search   |
| 2   | イベント分割 F1 | >= 0.75          | Phase 2      | 手動アノテーション   |
| 3   | 挿入遅延 p99    | <= 5ms           | Phase 1      | Mem0 add()           |
| 4   | 検索遅延 p99    | <= 20ms (10K)    | Phase 3      | Mem0 search()        |
| 5   | RSS メモリ      | Mem0 比 50% 以下 | Phase 3      | Mem0 同等データ      |
| 6   | LoCoMo 精度     | >= 68.5% (Mem0)  | Phase 3+     | Mem0 / Letta / MemOS |
