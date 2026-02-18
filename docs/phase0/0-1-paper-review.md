# 0-1. 論文精読ノート

Phase 0 研究タスク。各論文の要点・shabti への適用可否・不採用理由をまとめる。

---

## 0-1a. GAM — Generative Agents: Interactive Simulacra of Human Behavior

**著者**: Park et al. (Stanford / Google DeepMind, 2023)
**発表**: UIST 2023 | arXiv:2304.03442

### 要点

25体の LLM エージェントをサンドボックス環境に配置し、自律的な社会行動を実現。
メモリシステムは 3 要素で構成:

1. **Memory Stream**: 自然言語による append-only ログ。各レコードは ConceptNode (subject/predicate/object トリプル + embedding + poignancy スコア + keywords)
2. **Three-Factor Retrieval**: `score = w_recency * recency + w_relevance * relevance + w_importance * importance`
3. **Reflection**: 累積 importance が閾値 (150) を超えると LLM が焦点質問を生成 → メモリ検索 → 高次の洞察を生成し memory stream に書き戻す

### 重要な技術詳細

**検索スコアリング (コードから判明した実際の重み)**:

```
score = 0.5 * recency + 3.0 * relevance + 2.0 * importance
```

論文では「等重み」と主張しているが、コード上のグローバル重み `gw = [0.5, 3, 2]` が支配的。relevance (コサイン類似度) が recency の 6 倍の影響力。

**Recency**: `decay_factor^rank` (位置ベース減衰)。コード上は 0.99、論文は 0.995 と不一致。
**Importance**: 記憶作成時に LLM が 1-10 の整数で評価。一度付与されたら変更されない。
**Relevance**: 事前計算済み embedding のコサイン類似度。クエリ時は embedding 1 回のみで LLM 不要。

**Reflection トリガー**: 新規記憶の poignancy 累積が 150 を超えると発火 (シミュレーション日あたり 2-3 回)。

### shabti への適用

| GAM の要素                  | shabti での対応                                   | 適用判断                                                         |
| --------------------------- | ------------------------------------------------- | ---------------------------------------------------------------- |
| Three-Factor Retrieval      | Qdrant ANN + Rust での後処理スコアリング          | **採用** — LLM 不要で実装可能                                    |
| Recency decay               | `decay_factor^elapsed_hours` (真の時間減衰)       | **採用 (改良)** — 位置ベースではなく時間ベース                   |
| Importance scoring          | fastembed-rs ヒューリスティクス or ローカル分類器 | **採用 (改良)** — LLM 不要、多次元化、動的更新可能に             |
| Reflection                  | HDBSCAN クラスタリング                            | **代替採用** — クラスタ重心 = LLM 不要の「自動的リフレクション」 |
| Evidence pointers (filling) | petgraph の型付きエッジ (因果、時間、証拠)        | **採用 (拡張)** — フラットリストからグラフ構造へ                 |
| Memory Stream (append-only) | Append-Only Log + Qdrant                          | **採用**                                                         |

### JIT コンテキスト組立 (LLM なし)

GAM の検索はクエリ時に LLM を呼ばない設計になっており、shabti で完全再現可能:

1. fastembed-rs でクエリ embedding 生成
2. Qdrant で top-K ANN 検索 (relevance)
3. Rust で recency スコア計算 (`decay^elapsed_hours`)
4. Qdrant payload から importance 読み取り
5. 三要素加重スコアで再ランキング

### 不採用理由

- **位置ベース recency**: 時間情報を失う。真の時間減衰に置き換え
- **固定整数 importance (1-10)**: 粗すぎ、静的。多次元 + 動的更新に
- **忘却なし**: メモリ無限成長。有効期限 + クラスタ統合で対応
- **モノリシック設計**: メモリ + 推論 + 計画が密結合。shabti はメモリエンジンのみ
- **ハードコード閾値 (150)**: 適応的閾値に変更

---

## 0-1b. EM-LLM — Human-Inspired Episodic Memory for Infinite Context LLMs

**著者**: Fountas et al. (2024)
**発表**: ICLR 2025 | arXiv:2407.09450

### 要点

Event Segmentation Theory (EST) に基づき、LLM の KV キャッシュをエピソード記憶イベントに分割。
ファインチューニング不要、推論時のみで動作。

### ベイズ的サプライズによるイベント分割

**数学的定式化**:

```
S(x_t) = -log P(x_t | x_1, ..., x_{t-1}; theta)
```

イベント境界の検出条件:

```
S(x_t) > T
T = mean(S over window) + gamma * std(S over window)
```

- `gamma` = 1.0 (デフォルト): 標準偏差スケーリング
- `min_block_size` = 8, `max_block_size` = 128
- **適応的閾値**: 移動窓の統計量に基づくため、局所的なコンテキスト特性に自動適応

### Graph-Theoretic Boundary Refinement (Phase 2)

初期境界をアテンションキーベクトルの dot product 類似度行列で精緻化:

- **Modularity**: `f_M(A, B) = (1/4m) * SUM [A_ij - k_i*k_j/2m] * delta(c_i, c_j)` を最大化
- **Conductance**: 境界をまたぐエッジの割合を最小化
- Modularity 精緻化で初期分割の約 2 倍の品質向上

### コサイン距離閾値との比較

| 観点     | Surprise ベース                     | コサイン距離                |
| -------- | ----------------------------------- | --------------------------- |
| 信号源   | LLM 自身の予測分布 (予測的不連続)   | Embedding 間の表面的距離    |
| コスト   | 自己回帰推論のバイプロダクト (O(1)) | Embedding 生成 + 比較が必要 |
| 閾値     | 自己校正 (mean + gamma \* std)      | 手動チューニングが必要      |
| 適用条件 | 自己回帰モデルが必要                | 任意の embedding に適用可能 |

### shabti への適用

| EM-LLM の要素                    | 適用判断     | 理由                                                                               |
| -------------------------------- | ------------ | ---------------------------------------------------------------------------------- |
| Surprise ベース分割              | **不採用**   | shabti はエンコーダ embedding を使用し、自己回帰予測分布がない                     |
| 適応的閾値 (mean + gamma \* std) | **採用**     | 固定閾値から `T = mean(D) + gamma * std(D)` に変更。1 行の変更で自己校正境界を実現 |
| Graph-theoretic refinement       | **将来検討** | Embedding 類似度行列で modularity 最適化。中程度の実装コストで最大の品質向上       |
| Contiguity buffer                | **採用**     | 類似度検索でヒットしたイベントの隣接イベントも取得。FIFO で自然な時間減衰          |
| Representative token selection   | **不採用**   | アテンションスコアが不要。イベントの平均 embedding を代表として使用                |

### 最重要な知見

**適応的閾値の導入が最もコスパの高い改善**。shabti の現在のコサイン距離固定閾値を `mean + gamma * std` に置き換えるだけで、データ特性に自動適応する境界検出が実現できる。

---

## 0-1c. A-MEM — Agentic Memory for LLM Agents

**著者**: Xu et al. (2025)
**発表**: NeurIPS 2025 (poster) | arXiv:2502.12110

### 要点

Zettelkasten 原則を LLM エージェントメモリに適用。各記憶は「原子的ノート」として構造化メタデータ + 双方向リンクを持ち、新規ノート追加時に既存ノートが「進化」する。

### メモリノートデータモデル (7-tuple)

```
m_i = { c_i, t_i, K_i, G_i, X_i, e_i, L_i }
```

| フィールド | 型        | 説明                                     | 生成方法   |
| ---------- | --------- | ---------------------------------------- | ---------- |
| c_i        | string    | 原文コンテンツ                           | 入力から   |
| t_i        | timestamp | タイムスタンプ                           | 入力から   |
| K_i        | string[]  | キーワード (重要度順、最低3つ)           | LLM 生成   |
| G_i        | string[]  | カテゴリタグ                             | LLM 生成   |
| X_i        | string    | 1 文の文脈説明                           | LLM 生成   |
| e_i        | float[]   | embedding (`encode(concat(c, K, G, X))`) | エンコーダ |
| L_i        | set\<id\> | リンク先ノート ID                        | LLM 決定   |

**重要な設計選択**: embedding は raw content + 全メタデータの連結に対して計算 → ベクトル空間が LLM の高次解釈を反映 (**enriched embedding**)。

### 自己組織化メカニズム (3 段階)

1. **Note Construction**: LLM がキーワード・タグ・文脈を生成 → enriched embedding 計算
2. **Link Generation**: k-NN (k=10) で候補選択 → LLM がリンク判断 (2 段階ハイブリッド)
3. **Memory Evolution**: 近傍ノートの K/G/X を LLM が更新 → embedding 再計算

**1 挿入あたり LLM 呼び出し**: 12 回 (P_s1 + P_s2 + P_s3 x 近傍数)。ただし合計 1,200-2,500 トークンと効率的。

### shabti への適用

| A-MEM の要素             | 適用判断                  | shabti での実装                                                                                                |
| ------------------------ | ------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Enriched embedding       | **採用 (最重要)**         | `encode(concat(content, keywords, tags))` — ゼロコストで embedding 品質向上                                    |
| 7-field ノート構造       | **採用 (拡張)**           | resolution level + provenance + edge weight を追加                                                             |
| 2 段階リンク生成         | **採用 (Stage 2 を変更)** | Stage 1: Qdrant k-NN。Stage 2: 複合スコア (`cosine + keyword_jaccard + tag_overlap > threshold`) で LLM 不要に |
| 双方向リンク             | **採用**                  | petgraph で双方向エッジ                                                                                        |
| Memory Evolution         | **採用 (LLM 不要化)**     | キーワード集合のマージ、タグ伝播、embedding 再計算を決定的に                                                   |
| LLM 依存のメタデータ生成 | **不採用**                | RAKE/YAKE/KeyBERT でローカル抽出                                                                               |

### 不採用理由

- **挿入ごとの 12 LLM 呼び出し**: ローカルファーストで数百ノート処理には不可能
- **意味ドリフトの無制限**: 進化でノートの embedding が原文から乖離。shabti はドリフト距離の閾値で制御すべき
- **剪定/忘却なし**: 1M ノートで 1.4GB。TTL + アーカイバル必要
- **1-hop しか使わない検索**: リンク構造の価値を活かしていない。shabti は petgraph の多ホップ走査と組み合わせるべき

---

## 0-1d. Memory in the Age of AI Agents (サーベイ)

**著者**: Hu et al. (47名共著, 2025)
**発表**: arXiv:2512.13564

### メモリ分類法 (3 次元)

**Dimension 1 — Forms (物理的実装)**:

- **Token-level**: テキスト等の離散的ユニット。Flat (1D) / Planar (2D: グラフ) / **Hierarchical (3D: 多層構造)**
- **Parametric**: モデル重みにエンコード (LoRA 等)
- **Latent**: KV キャッシュ、hidden activation 等の暗黙的状態

**Dimension 2 — Functions (認知的目的)**:

- **Factual**: 宣言的知識 (ユーザー情報、ドメイン知識)
- **Experiential**: 手続き的知識 (case-based / strategy-based / skill-based)
- **Working**: 動的スクラッチパッド (コンテキスト内推論)

**Dimension 3 — Dynamics (ライフサイクル)**:

- **Formation**: 要約・蒸留・構造化
- **Evolution**: 統合・更新・忘却・矛盾解決
- **Retrieval**: タイミング判断 → クエリ構築 → 戦略選択 → 後処理

### shabti の位置づけ

> サーベイ上で shabti は独自のポジションを占める

| 次元      | shabti のポジション                                         |
| --------- | ----------------------------------------------------------- |
| Form      | Token-level with **Hierarchical (3D)** topology (L0-L3)     |
| Function  | 主に Factual Memory。Phase B で Experiential 追加予定       |
| Formation | Embedding シフト検出 (イベント分割) + k-NN リンク + HDBSCAN |
| Evolution | Append-Only + 時間減衰 + supersedes チェーン                |
| Retrieval | 多段検索 (L3→L0) + semantic + temporal + graph + DSL        |

**差別化ポイント**:

1. **唯一の完全アルゴリズム駆動メモリシステム** — 他の全システム (Mem0, MemOS, Letta, A-MEM, GAM) は LLM 依存
2. **唯一の Rust ネイティブ** — 全調査対象が Python
3. **唯一の数学的粗視化による階層化** — 他は LLM 要約で抽象化
4. **唯一のタイムトラベル機能** — CoW スナップショット
5. **唯一のゼロ情報損失設計** — Append-Only + soft forgetting

### 見落としているアプローチ

1. **RL ベースメモリ管理 (Mem-alpha, Memory-R1)**: 閾値・減衰率・クラスタパラメータを RL で最適化。Phase B+ で検討
2. **生成的メモリ合成**: 検索結果からのテンプレートベース合成 (LLM 不要でも可)
3. **Experiential Memory サブタイプ**: case/strategy/skill ベースの手続き記憶
4. **メモリポイズニング防御**: 悪意ある入力がセッション横断で汚染するリスク
5. **並行制御 (CRDT)**: 複数エージェントの共有メモリ

---

## 0-1e. MemOS — Memory Operating System

**著者**: MemTensor team (2025)
**発表**: arXiv:2505.22101 (short) / arXiv:2507.03724 (full)

### MemCube 抽象化

全メモリを統一的にカプセル化するコンテナ:

**Memory Payload** (3 タイプ):

- Plaintext (文書、KG、プロンプト)
- Activation states (KV キャッシュ、hidden activation)
- Parameter deltas (LoRA モジュール)

**Metadata Header** (3 カテゴリ):

- Descriptive: タイムスタンプ、Origin Signature、Semantic Type
- Governance: アクセス制御、Lifespan Policy (TTL, 減衰)、Priority、Sensitivity
- Behavioral: Access Frequency、Recency Metrics、Contextual Fingerprint、Version Chain

### 3 層アーキテクチャ

| 層             | コンポーネント                                      | 役割                                                        |
| -------------- | --------------------------------------------------- | ----------------------------------------------------------- |
| Interface      | MemReader, Memory API, Memory Pipeline              | NL 解析、API (Provenance/Update/LogQuery)、DAG パイプライン |
| Operation      | MemScheduler, MemLifecycle, MemOperator             | 動的型選択、5 状態ライフサイクル、タグ/グラフ管理           |
| Infrastructure | MemGovernance, MemVault, MemLoader/Dumper, MemStore | 権限、ストレージ、マイグレーション、pub/sub                 |

**ライフサイクル**: Generated → Activated → Merged → Archived → Expired (+ ロールバック、凍結)

### ベンチマーク結果

LoCoMo で OpenAI のグローバルメモリ比 **159% 改善** (temporal reasoning)。
PreFEval, PersonaMem, LongMemEval, LoCoMo の全てで 1 位 (Mem0, Zep, Memobase 等を上回る)。

### shabti への適用

| MemOS の要素                                 | 適用判断         | 理由                                                          |
| -------------------------------------------- | ---------------- | ------------------------------------------------------------- |
| MemCube の Behavioral Indicators             | **採用**         | contextual fingerprint, version chain を MemoryEntry に追加   |
| ライフサイクル状態                           | **採用**         | active / merged / archived / superseded をフィールド追加      |
| Provenance API パターン                      | **採用**         | origin_type (CLI 入力 / エージェント生成 / インポート) の記録 |
| MemoryPathResolver (topic-concept-fact 分解) | **参考**         | 多解像度検索のクエリ計画に応用                                |
| Activation/Parameter memory                  | **不採用**       | shabti は LLM 推論状態を管理しない (設計原則)                 |
| NL ベース MemReader                          | **不採用**       | CLI/DSL による決定的ルーティング                              |
| Plaintext ↔ Activation ↔ Parameter 変換      | **不採用**       | ファインチューニング基盤はスコープ外                          |
| Pub/sub MemStore                             | **不採用 (MVP)** | Phase B の MCP サーバーで再検討                               |

---

## 横断的まとめ: shabti への主要知見

### 最優先で採用すべき要素

1. **Enriched Embedding** (A-MEM): `encode(content + keywords + tags)` — ゼロコストで embedding 品質向上
2. **適応的閾値** (EM-LLM): `T = mean(D) + gamma * std(D)` — 1 行の変更で自己校正境界検出
3. **Three-Factor Scoring** (GAM): `score = w_r * recency + w_v * relevance + w_i * importance` — 真の時間減衰で
4. **Contiguity Buffer** (EM-LLM): 検索ヒットの隣接イベントも自動取得
5. **ライフサイクル状態** (MemOS): active / merged / archived / superseded

### 設計原則の確認

全 5 論文の分析を通じて、shabti の「LLM 不要・ローカルファースト・Rust ネイティブ」という設計方針が、既存システムに対する明確な差別化要因であることが確認された。既存システムの知見を LLM 不要で再現する方法が各論文から具体的に導出できた。
