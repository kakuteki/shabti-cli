# 0-2. 競合ソースコード調査

Phase 0 研究タスク。Mem0, Letta, MemOS のデータモデル・API 設計を比較分析する。

---

## 0-2a. Mem0

**リポジトリ**: https://github.com/mem0ai/mem0
**Stars**: ~47,500 | **言語**: Python
**概要**: "Universal memory layer for AI Agents"

### データモデル

```python
class MemoryItem(BaseModel):
    id: str                          # UUID
    memory: str                      # 抽出されたファクト/記憶テキスト
    hash: Optional[str]              # MD5 (重複検出用だが未使用)
    metadata: Optional[Dict]         # ユーザー提供メタデータ
    score: Optional[float]           # 類似度スコア (検索時)
    created_at: Optional[str]        # ISO タイムスタンプ (US/Pacific 固定)
    updated_at: Optional[str]        # 更新タイムスタンプ
```

Vector Store payload:

- `data`, `hash`, `created_at`, `updated_at`
- `user_id`, `agent_id`, `run_id` (セッションスコーピング)
- `actor_id`, `role` (発話者)

### ハイブリッドストア設計

| ストア                  | 目的                                                           | 実装                                        |
| ----------------------- | -------------------------------------------------------------- | ------------------------------------------- |
| **Vector Store**        | 主記憶 (テキスト + embedding + メタデータ)。全 CRUD はここ経由 | Qdrant (デフォルト) 他 22+ バックエンド     |
| **Graph Store**         | エンティティ + 関係性のナレッジグラフ                          | Neo4j (デフォルト), Memgraph, Neptune, Kuzu |
| **SQLite (History DB)** | 全変更の append-only 監査ログ                                  | SQLite                                      |

**並行処理**: `add()` 時に Vector と Graph を `ThreadPoolExecutor` で並列実行。

### API 設計

| メソッド                                                                | 説明                             |
| ----------------------------------------------------------------------- | -------------------------------- |
| `add(messages, *, user_id, agent_id, run_id, metadata, infer=True)`     | メッセージからファクト抽出・保存 |
| `search(query, *, user_id, limit=100, filters, threshold, rerank=True)` | セマンティック検索               |
| `get(memory_id)`                                                        | ID で取得                        |
| `get_all(*, user_id, limit=100)`                                        | スコープ内全取得                 |
| `update(memory_id, data)`                                               | 直接更新                         |
| `delete(memory_id)` / `delete_all(user_id)`                             | 削除                             |
| `history(memory_id)`                                                    | 変更履歴                         |

**検索フィルタ構文**: `{"key": {"gt": 10}}`, `{"AND": [...]}`, `{"key": {"contains": "text"}}` 等。

### LLM 依存性 (致命的)

| 操作               | LLM 呼び出し回数 | 目的                                       |
| ------------------ | ---------------- | ------------------------------------------ |
| `add()` (vector)   | 2                | ファクト抽出 + ADD/UPDATE/DELETE/NONE 判断 |
| `add()` (graph)    | 3                | エンティティ抽出 + 関係性抽出 + 削除検出   |
| `search()` (graph) | 1                | クエリからのエンティティ抽出               |

**1 回の `add()` で最大 5 LLM 呼び出し** (graph 有効時)。

### shabti との差異

| Mem0 の弱点                            | shabti の機会            |
| -------------------------------------- | ------------------------ |
| 全操作が LLM 必須 (1-10 秒/操作)       | LLM 不要で基本操作 < 1ms |
| Python のみ                            | Rust ネイティブ + FFI    |
| 時間減衰/TTL なし                      | 組み込み時間減衰         |
| FAISS フィルタがナイーブ (post-search) | Qdrant の pre-filter     |
| Importance/Recency スコアリングなし    | Three-Factor Scoring     |
| タイムゾーンハードコード (US/Pacific)  | UTC ベース               |

---

## 0-2b. Letta (formerly MemGPT)

**リポジトリ**: https://github.com/letta-ai/letta
**Stars**: ~21,200 | **言語**: Python
**概要**: OS 風の階層的メモリ管理 + LLM のセルフエディット

### メモリアーキテクチャ (4 層)

| 層                  | 比喩         | 説明                                                                       | 永続化                            |
| ------------------- | ------------ | -------------------------------------------------------------------------- | --------------------------------- |
| **Core Memory**     | RAM          | システムプロンプトに固定。常にコンテキスト内 (`persona`, `human` ブロック) | PostgreSQL                        |
| **Message Buffer**  | Cache        | 最近のメッセージ。`max_message_buffer_length` で管理                       | PostgreSQL + Vector DB            |
| **Recall Memory**   | Disk         | 全会話履歴。日付・テキスト検索可能                                         | SQL + Vector                      |
| **Archival Memory** | Deep Storage | 無制限容量の長期知識ストア                                                 | pgvector / Turbopuffer / Pinecone |

### Block データモデル

```python
class Block:
    id: str
    label: str          # "human", "persona" 等
    value: str          # テキスト内容
    limit: int          # 文字数制限 (~2000)
    description: str    # 目的の説明
    read_only: bool     # エージェントによる変更防止
    metadata: dict
    tags: List[str]
    version: int        # 楽観的ロック用
```

### メモリバージョニング

**Legacy (Letta Server)**: `Block` に `version` カラム + `BlockHistory` テーブル。楽観的ロック (version mismatch → 409 Conflict)。Last-write-wins。

**MemFS / Context Repositories (Letta Code v0.15+)**:

- **Git ベースのメモリファイルシステム**: `~/.letta/agents/<id>/memory/` にマークダウンファイル
- **Commit**: エージェントが明示的に commit/push して保存
- **Branching**: git worktrees でサブエージェント並列作業
- **Merging**: git merge で矛盾解決
- **Rollback**: git 履歴で任意時点に復元

### コンテキストウィンドウ管理

- **Sliding Window Compaction**: `sliding_window_percentage` (e.g., 0.3) で最新 70% を保持
- 古いメッセージは再帰的要約 (LLM 依存)
- データは削除されない (DB に残り Recall Memory で検索可能)

### セルフエディット (ツール呼び出し)

```
core_memory_append(label, content)
core_memory_replace(label, old_content, new_content)
archival_memory_insert(content)
archival_memory_search(query)
conversation_search(query)
```

LLM が自律的にこれらのツールを呼び出してメモリを管理。

### Hybrid Search

**Reciprocal Rank Fusion (RRF)**: セマンティック検索 (pgvector) + キーワード検索 (PostgreSQL full-text) の結果をランク融合。

### Sleep-time Compute

バックグラウンドエージェントが会話履歴を非同期処理し、"learned context" でメモリブロックを更新。レスポンスレイテンシから分離。

### shabti との差異

| Letta の弱点                                | shabti の機会                               |
| ------------------------------------------- | ------------------------------------------- |
| LLM 必須 (メモリ管理自体に LLM 推論)        | アルゴリズム駆動 + オプショナル LLM         |
| サーバー前提 (PostgreSQL / Docker)          | 完全ローカル、ゼロ依存                      |
| Python、組み込み不可                        | Rust、FFI で任意言語から利用可能            |
| イベント分割なし (線形メッセージ列)         | コサイン距離/適応的閾値によるエピソード境界 |
| 多解像度インデックスなし (フラットベクトル) | L0-L3 階層的検索                            |
| 粗い Compaction (再帰的要約で詳細喪失)      | イベントベースの選択的保存                  |
| Last-write-wins ブロック更新                | Append-Only + CRDT 検討                     |

---

## 0-2c. MemOS

**リポジトリ**: https://github.com/MemTensor/MemOS
**概要**: LLM メモリの OS レベルガバナンス

### MemCube API インターフェース

| API                | 操作                                             | スコープ         |
| ------------------ | ------------------------------------------------ | ---------------- |
| **Provenance API** | メタデータ埋め込み、origin 追跡                  | トレーサビリティ |
| **Update API**     | append, merge, overwrite (バージョン対応)        | 変更             |
| **LogQuery API**   | タイムスタンプ・呼び出し元・操作タイプでフィルタ | 監査             |

### 3 層設計の詳細

- **Interface**: NL パーサー (MemReader) + 構造化 API + DAG パイプライン
- **Operation**: MemScheduler (LRU/類似度/ラベルベース選択) + MemLifecycle (5 状態マシン) + MemOperator (タグ/グラフ管理)
- **Infrastructure**: MemGovernance (権限) + MemVault (統合ストレージ) + MemStore (pub/sub)

### shabti との差異

| MemOS の弱点                                          | shabti の機会                    |
| ----------------------------------------------------- | -------------------------------- |
| Activation/Parameter memory は LLM ランタイムと密結合 | LLM アグノスティック設計         |
| NL ベースのクエリ解析 (LLM 必須)                      | CLI/DSL による決定的ルーティング |
| エンタープライズ向けガバナンス (過剰)                 | 開発者向け最小限ガバナンス       |

---

## 比較表

| 観点                   | Mem0                    | Letta                 | MemOS                 | **shabti (計画)**                       |
| ---------------------- | ----------------------- | --------------------- | --------------------- | --------------------------------------- |
| **言語**               | Python                  | Python                | Python                | **Rust**                                |
| **LLM 依存**           | 必須 (2-5 calls/add)    | 必須 (推論ベース)     | 必須 (MemReader)      | **不要**                                |
| **ベクトル DB**        | 22+ バックエンド        | pgvector              | FAISS/Milvus          | **Qdrant (Embedded)**                   |
| **グラフ**             | Neo4j (LLM 抽出)        | なし                  | MemOperator           | **petgraph (k-NN 自動リンク)**          |
| **イベント分割**       | なし                    | なし                  | なし                  | **コサイン距離 + 適応的閾値**           |
| **多解像度**           | なし                    | なし                  | MemCube 型変換        | **L0-L3 embedding 集約**                |
| **クラスタリング**     | なし                    | なし                  | なし                  | **HDBSCAN**                             |
| **重複排除**           | LLM 判断                | なし                  | 不明                  | **SHA-256 + コサイン > 0.98**           |
| **時間減衰**           | なし                    | なし                  | Lifespan Policy       | **`score * decay(age) * access_boost`** |
| **矛盾検出**           | LLM 判断 (DELETE)       | なし                  | なし                  | **同一クラスタ内差異検出**              |
| **スナップショット**   | なし                    | Git (Letta Code のみ) | Version Chain         | **CoW スナップショット**                |
| **Namespace**          | user_id/agent_id/run_id | Shared blocks         | Access Control        | **Private/Shared/Global**               |
| **検索**               | Vector + Graph          | Hybrid (RRF)          | Semantic + Structural | **Multi-resolution + DSL**              |
| **ローカルファースト** | 部分的 (FAISS)          | SQLite (dev のみ)     | 不明                  | **完全ローカル**                        |
| **ベンチマーク比較**   | LoCoMo 68.5%            | LoCoMo 74.0%          | LoCoMo 1位            | **目標: LoCoMo 上位**                   |

### 結論

3 プロジェクトとも LLM 依存がコア設計に組み込まれている。shabti の差別化は「同等以上のメモリ管理品質を LLM 不要で実現」することにある。具体的には:

1. **Mem0 の LLM ファクト抽出** → shabti の enriched embedding + ヒューリスティクス
2. **Letta の LLM セルフエディット** → shabti の自動イベント分割 + k-NN リンク
3. **MemOS の NL クエリ解析** → shabti の Composable Query DSL
