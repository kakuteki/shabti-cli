use std::collections::{HashSet, VecDeque};

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

/// A memory node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryNode {
    pub id: String,
    pub content: String,
    pub created_at: u64,
}

/// An edge representing similarity between two memory nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilarityEdge {
    pub weight: f32,
}

/// Serialization helper for the graph.
#[derive(Serialize, Deserialize)]
struct SerializableGraph {
    nodes: Vec<MemoryNode>,
    edges: Vec<(usize, usize, SimilarityEdge)>,
}

/// The memory graph backed by petgraph.
pub struct MemoryGraph {
    graph: DiGraph<MemoryNode, SimilarityEdge>,
}

impl MemoryGraph {
    /// Create a new empty memory graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
        }
    }

    /// Add a memory node. Returns the NodeIndex.
    pub fn add_node(&mut self, node: MemoryNode) -> NodeIndex {
        self.graph.add_node(node)
    }

    /// Add a similarity edge between two nodes.
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, weight: f32) {
        self.graph
            .add_edge(from, to, SimilarityEdge { weight });
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// BFS traversal from a seed node, returning nodes within `max_depth` hops.
    pub fn bfs(&self, seed: NodeIndex, max_depth: usize) -> Vec<NodeIndex> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        visited.insert(seed);
        queue.push_back((seed, 0));

        while let Some((node, depth)) = queue.pop_front() {
            result.push(node);
            if depth < max_depth {
                for neighbor in self.graph.neighbors(node) {
                    if visited.insert(neighbor) {
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        result
    }

    /// DFS traversal from a seed node, returning nodes within `max_depth` hops.
    pub fn dfs(&self, seed: NodeIndex, max_depth: usize) -> Vec<NodeIndex> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut stack = vec![(seed, 0)];

        while let Some((node, depth)) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            result.push(node);
            if depth < max_depth {
                for neighbor in self.graph.neighbors(node) {
                    if !visited.contains(&neighbor) {
                        stack.push((neighbor, depth + 1));
                    }
                }
            }
        }

        result
    }

    /// Get a reference to a node's data.
    pub fn get_node(&self, idx: NodeIndex) -> Option<&MemoryNode> {
        self.graph.node_weight(idx)
    }

    /// Serialize the graph to JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        let nodes: Vec<MemoryNode> = self
            .graph
            .node_indices()
            .map(|i| self.graph[i].clone())
            .collect();
        let edges: Vec<(usize, usize, SimilarityEdge)> = self
            .graph
            .edge_indices()
            .map(|e| {
                let (src, dst) = self.graph.edge_endpoints(e).unwrap();
                (src.index(), dst.index(), self.graph[e].clone())
            })
            .collect();
        let sg = SerializableGraph { nodes, edges };
        Ok(serde_json::to_string(&sg)?)
    }

    /// Deserialize a graph from JSON.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let sg: SerializableGraph = serde_json::from_str(json)?;
        let mut graph = DiGraph::new();
        for node in sg.nodes {
            graph.add_node(node);
        }
        for (src, dst, edge) in sg.edges {
            graph.add_edge(NodeIndex::new(src), NodeIndex::new(dst), edge);
        }
        Ok(Self { graph })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node(id: &str, content: &str) -> MemoryNode {
        MemoryNode {
            id: id.to_string(),
            content: content.to_string(),
            created_at: 1000,
        }
    }

    #[test]
    fn new_graph_is_empty() {
        let g = MemoryGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn add_nodes_increments_count() {
        let mut g = MemoryGraph::new();
        g.add_node(sample_node("a", "hello"));
        assert_eq!(g.node_count(), 1);
        g.add_node(sample_node("b", "world"));
        assert_eq!(g.node_count(), 2);
    }

    #[test]
    fn add_edge_increments_count() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", "hello"));
        let b = g.add_node(sample_node("b", "world"));
        g.add_edge(a, b, 0.9);
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn get_node_returns_data() {
        let mut g = MemoryGraph::new();
        let idx = g.add_node(sample_node("a", "hello"));
        let node = g.get_node(idx).expect("node should exist");
        assert_eq!(node.id, "a");
        assert_eq!(node.content, "hello");
    }

    #[test]
    fn get_node_invalid_index_returns_none() {
        let g = MemoryGraph::new();
        let fake_idx = NodeIndex::new(999);
        assert!(g.get_node(fake_idx).is_none());
    }

    // ── BFS tests ────────────────────────────────────────────────────────

    #[test]
    fn bfs_returns_seed_at_depth_zero() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", "seed"));
        let result = g.bfs(a, 0);
        assert_eq!(result, vec![a]);
    }

    #[test]
    fn bfs_finds_direct_neighbors() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", "seed"));
        let b = g.add_node(sample_node("b", "neighbor1"));
        let c = g.add_node(sample_node("c", "neighbor2"));
        let d = g.add_node(sample_node("d", "distant"));
        g.add_edge(a, b, 0.9);
        g.add_edge(a, c, 0.8);
        g.add_edge(c, d, 0.7);

        let result = g.bfs(a, 1);
        assert!(result.contains(&a));
        assert!(result.contains(&b));
        assert!(result.contains(&c));
        assert!(!result.contains(&d), "depth-2 node should not be included at depth 1");
    }

    #[test]
    fn bfs_respects_max_depth() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", ""));
        let b = g.add_node(sample_node("b", ""));
        let c = g.add_node(sample_node("c", ""));
        let d = g.add_node(sample_node("d", ""));
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, d, 1.0);

        let depth1 = g.bfs(a, 1);
        let depth2 = g.bfs(a, 2);
        let depth3 = g.bfs(a, 3);

        assert_eq!(depth1.len(), 2); // a, b
        assert_eq!(depth2.len(), 3); // a, b, c
        assert_eq!(depth3.len(), 4); // a, b, c, d
    }

    // ── DFS tests ────────────────────────────────────────────────────────

    #[test]
    fn dfs_returns_seed_at_depth_zero() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", "seed"));
        let result = g.dfs(a, 0);
        assert_eq!(result, vec![a]);
    }

    #[test]
    fn dfs_finds_reachable_nodes() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", ""));
        let b = g.add_node(sample_node("b", ""));
        let c = g.add_node(sample_node("c", ""));
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);

        let result = g.dfs(a, 2);
        assert!(result.contains(&a));
        assert!(result.contains(&b));
        assert!(result.contains(&c));
    }

    #[test]
    fn dfs_respects_max_depth() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", ""));
        let b = g.add_node(sample_node("b", ""));
        let c = g.add_node(sample_node("c", ""));
        let d = g.add_node(sample_node("d", ""));
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, d, 1.0);

        let depth1 = g.dfs(a, 1);
        assert_eq!(depth1.len(), 2); // a, b
        assert!(!depth1.contains(&c));
    }

    // ── Serialization tests ──────────────────────────────────────────────

    #[test]
    fn roundtrip_json_preserves_graph() {
        let mut g = MemoryGraph::new();
        let a = g.add_node(sample_node("a", "hello"));
        let b = g.add_node(sample_node("b", "world"));
        g.add_edge(a, b, 0.85);

        let json = g.to_json().expect("serialization should succeed");
        let g2 = MemoryGraph::from_json(&json).expect("deserialization should succeed");

        assert_eq!(g2.node_count(), 2);
        assert_eq!(g2.edge_count(), 1);
        assert_eq!(g2.get_node(NodeIndex::new(0)).unwrap().id, "a");
        assert_eq!(g2.get_node(NodeIndex::new(1)).unwrap().id, "b");
    }

    #[test]
    fn empty_graph_roundtrip() {
        let g = MemoryGraph::new();
        let json = g.to_json().expect("serialization should succeed");
        let g2 = MemoryGraph::from_json(&json).expect("deserialization should succeed");
        assert_eq!(g2.node_count(), 0);
        assert_eq!(g2.edge_count(), 0);
    }
}
