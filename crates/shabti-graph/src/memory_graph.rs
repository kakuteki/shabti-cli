use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::Path;

use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use shabti_core::error::{ShabtiError, ShabtiResult};

/// Serializable representation of the graph.
#[derive(Serialize, Deserialize)]
struct GraphData {
    nodes: Vec<Uuid>,
    edges: Vec<(Uuid, Uuid, f32)>,
}

/// A directed graph where nodes are memory entry UUIDs
/// and edges carry similarity weights.
pub struct MemoryGraph {
    graph: DiGraph<Uuid, f32>,
    uuid_to_node: HashMap<Uuid, NodeIndex>,
}

impl MemoryGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            uuid_to_node: HashMap::new(),
        }
    }

    /// Add a node if it doesn't already exist.
    pub fn add_node(&mut self, id: Uuid) {
        if !self.uuid_to_node.contains_key(&id) {
            let idx = self.graph.add_node(id);
            self.uuid_to_node.insert(id, idx);
        }
    }

    /// Add or update an edge between two nodes.
    /// Both nodes must already exist.
    pub fn add_edge(&mut self, from: Uuid, to: Uuid, weight: f32) {
        let from_idx = match self.uuid_to_node.get(&from) {
            Some(&idx) => idx,
            None => return,
        };
        let to_idx = match self.uuid_to_node.get(&to) {
            Some(&idx) => idx,
            None => return,
        };

        // Check if edge already exists
        if let Some(edge_idx) = self.find_edge(from_idx, to_idx) {
            // Update weight
            self.graph[edge_idx] = weight;
        } else {
            self.graph.add_edge(from_idx, to_idx, weight);
        }
    }

    /// Remove a node and all its connected edges.
    pub fn remove_node(&mut self, id: Uuid) {
        if let Some(idx) = self.uuid_to_node.remove(&id) {
            self.graph.remove_node(idx);
            // Rebuild uuid_to_node since petgraph may reindex
            self.rebuild_index();
        }
    }

    /// BFS neighbors up to max_depth, returning (uuid, best_weight) pairs.
    /// Excludes the seed node itself.
    pub fn neighbors(&self, seed: Uuid, max_depth: usize) -> Vec<(Uuid, f32)> {
        let seed_idx = match self.uuid_to_node.get(&seed) {
            Some(&idx) => idx,
            None => return Vec::new(),
        };

        let mut visited: HashMap<NodeIndex, f32> = HashMap::new();
        let mut queue: VecDeque<(NodeIndex, usize, f32)> = VecDeque::new();

        // Start with the seed's direct neighbors
        for edge in self.graph.edges(seed_idx) {
            let target = edge.target();
            let weight = *edge.weight();
            if target != seed_idx && !visited.contains_key(&target) {
                visited.insert(target, weight);
                queue.push_back((target, 1, weight));
            }
        }

        while let Some((node, depth, _)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.graph.edges(node) {
                let target = edge.target();
                let weight = *edge.weight();
                if target != seed_idx && !visited.contains_key(&target) {
                    visited.insert(target, weight);
                    queue.push_back((target, depth + 1, weight));
                }
            }
        }

        visited
            .into_iter()
            .map(|(idx, weight)| (self.graph[idx], weight))
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> ShabtiResult<String> {
        let nodes: Vec<Uuid> = self
            .graph
            .node_indices()
            .map(|idx| self.graph[idx])
            .collect();

        let edges: Vec<(Uuid, Uuid, f32)> = self
            .graph
            .edge_indices()
            .map(|idx| {
                let (src, tgt) = self.graph.edge_endpoints(idx).unwrap();
                (self.graph[src], self.graph[tgt], self.graph[idx])
            })
            .collect();

        let data = GraphData { nodes, edges };
        serde_json::to_string(&data).map_err(|e| ShabtiError::Serialization(e.to_string()))
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> ShabtiResult<Self> {
        let data: GraphData =
            serde_json::from_str(json).map_err(|e| ShabtiError::Serialization(e.to_string()))?;

        let mut graph = Self::new();
        for uuid in data.nodes {
            graph.add_node(uuid);
        }
        for (from, to, weight) in data.edges {
            graph.add_edge(from, to, weight);
        }
        Ok(graph)
    }

    /// Save graph to a file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> ShabtiResult<()> {
        let path = path.as_ref();
        let json = self.to_json()?;

        // 一時ファイルに書いてからアトミックにリネーム
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, &json).map_err(ShabtiError::Io)?;
        fs::rename(&tmp_path, path).map_err(ShabtiError::Io)?;

        Ok(())
    }

    /// Load graph from a file. Returns empty graph if file doesn't exist.
    pub fn load<P: AsRef<Path>>(path: P) -> ShabtiResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = std::fs::read_to_string(path).map_err(ShabtiError::Io)?;
        Self::from_json(&json)
    }

    fn find_edge(&self, from: NodeIndex, to: NodeIndex) -> Option<EdgeIndex> {
        self.graph
            .edges(from)
            .find(|e| e.target() == to)
            .map(|e| e.id())
    }

    fn rebuild_index(&mut self) {
        self.uuid_to_node.clear();
        for idx in self.graph.node_indices() {
            self.uuid_to_node.insert(self.graph[idx], idx);
        }
    }
}

impl Default for MemoryGraph {
    fn default() -> Self {
        Self::new()
    }
}
