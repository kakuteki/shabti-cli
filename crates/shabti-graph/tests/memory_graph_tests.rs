use shabti_graph::MemoryGraph;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn new_graph_is_empty() {
    let graph = MemoryGraph::new();
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn add_node() {
    let mut graph = MemoryGraph::new();
    let id = Uuid::new_v4();
    graph.add_node(id);
    assert_eq!(graph.node_count(), 1);
}

#[test]
fn add_duplicate_node_is_idempotent() {
    let mut graph = MemoryGraph::new();
    let id = Uuid::new_v4();
    graph.add_node(id);
    graph.add_node(id); // duplicate
    assert_eq!(graph.node_count(), 1);
}

#[test]
fn add_edge() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_edge(a, b, 0.95);
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn add_edge_updates_weight() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_edge(a, b, 0.5);
    graph.add_edge(a, b, 0.9); // update weight
    assert_eq!(graph.edge_count(), 1);
    // Neighbors should show updated weight
    let neighbors = graph.neighbors(a, 1);
    assert_eq!(neighbors.len(), 1);
    assert!((neighbors[0].1 - 0.9).abs() < 1e-6);
}

#[test]
fn remove_node() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_edge(a, b, 0.8);
    graph.remove_node(a);
    assert_eq!(graph.node_count(), 1);
    assert_eq!(graph.edge_count(), 0);
}

#[test]
fn remove_nonexistent_node_no_panic() {
    let mut graph = MemoryGraph::new();
    graph.remove_node(Uuid::new_v4()); // should not panic
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn neighbors_bfs_depth_1() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_node(c);
    graph.add_edge(a, b, 0.9);
    graph.add_edge(b, c, 0.8);

    let neighbors = graph.neighbors(a, 1);
    // Depth 1: only b should be reached
    let ids: Vec<Uuid> = neighbors.iter().map(|(id, _)| *id).collect();
    assert!(ids.contains(&b));
    assert!(!ids.contains(&c));
}

#[test]
fn neighbors_bfs_depth_2() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_node(c);
    graph.add_edge(a, b, 0.9);
    graph.add_edge(b, c, 0.8);

    let neighbors = graph.neighbors(a, 2);
    // Depth 2: both b and c should be reached
    let ids: Vec<Uuid> = neighbors.iter().map(|(id, _)| *id).collect();
    assert!(ids.contains(&b));
    assert!(ids.contains(&c));
}

#[test]
fn neighbors_excludes_self() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_edge(a, b, 0.9);

    let neighbors = graph.neighbors(a, 1);
    let ids: Vec<Uuid> = neighbors.iter().map(|(id, _)| *id).collect();
    assert!(!ids.contains(&a), "self should be excluded from neighbors");
}

#[test]
fn neighbors_nonexistent_node_returns_empty() {
    let graph = MemoryGraph::new();
    let neighbors = graph.neighbors(Uuid::new_v4(), 1);
    assert!(neighbors.is_empty());
}

#[test]
fn json_roundtrip() {
    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_node(c);
    graph.add_edge(a, b, 0.9);
    graph.add_edge(b, c, 0.8);

    let json = graph.to_json().unwrap();
    let restored = MemoryGraph::from_json(&json).unwrap();

    assert_eq!(restored.node_count(), 3);
    assert_eq!(restored.edge_count(), 2);

    let neighbors = restored.neighbors(a, 1);
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].0, b);
}

#[test]
fn file_persistence_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("graph.json");

    let mut graph = MemoryGraph::new();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    graph.add_node(a);
    graph.add_node(b);
    graph.add_edge(a, b, 0.95);

    graph.save(&path).unwrap();
    let loaded = MemoryGraph::load(&path).unwrap();

    assert_eq!(loaded.node_count(), 2);
    assert_eq!(loaded.edge_count(), 1);
    let neighbors = loaded.neighbors(a, 1);
    assert_eq!(neighbors[0].0, b);
    assert!((neighbors[0].1 - 0.95).abs() < 1e-6);
}

#[test]
fn load_nonexistent_returns_empty_graph() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.json");
    let graph = MemoryGraph::load(&path).unwrap();
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn empty_graph_json_roundtrip() {
    let graph = MemoryGraph::new();
    let json = graph.to_json().unwrap();
    let restored = MemoryGraph::from_json(&json).unwrap();
    assert_eq!(restored.node_count(), 0);
    assert_eq!(restored.edge_count(), 0);
}
