# Shabti Benchmarks

Measured on real Qdrant (localhost:6334) + fastembed-rs (MultilingualE5Small, 384 dimensions).

## Environment

- OS: Windows 11 Pro
- Rust: release build (`--release`)
- Qdrant: Docker container (v1.13+), gRPC port 6334
- Embedding model: `MultilingualE5Small` (384-dim, local inference via ONNX Runtime)
- Corpus: 30 diverse English sentences

## Results

| Metric                  | Target   | Measured               | Status |
| ----------------------- | -------- | ---------------------- | ------ |
| **Recall@10**           | >= 0.85  | **1.000** (11/11 hits) | PASS   |
| **Insert latency p99**  | <= 100ms | **60.22ms**            | PASS   |
| **Insert latency p50**  | —        | 53.64ms                | —      |
| **Insert latency mean** | —        | 51.69ms                | —      |
| **Search latency p99**  | <= 100ms | **58.71ms**            | PASS   |
| **Search latency p50**  | —        | 51.43ms                | —      |
| **Search latency mean** | —        | 52.19ms                | —      |

## Notes

- Latency is dominated by local embedding computation (~50ms per text via ONNX Runtime).
  Qdrant round-trip adds ~1-10ms.
- Recall@10 achieves perfect score on the benchmark corpus, indicating strong semantic
  matching quality with the MultilingualE5Small model.
- Score explanation breakdown confirms `time_decay = 1.0` (freshly inserted) and
  `access_boost = 1.0` (no prior accesses), so composite score equals raw cosine similarity.
