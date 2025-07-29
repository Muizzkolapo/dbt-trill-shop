use anyhow::{Context, Result};
use petgraph::{Graph, Direction};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::debug;

/// Wrapper around petgraph DiGraph for lineage analysis
pub struct LineageGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
}

impl LineageGraph {
    pub fn new(graph: DiGraph<String, ()>, node_map: HashMap<String, NodeIndex>) -> Self {
        Self { graph, node_map }
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all descendants (downstream nodes) of a given node
    pub fn get_descendants(&self, node_id: &str) -> Result<Vec<String>> {
        let node_index = self.node_map
            .get(node_id)
            .context("Node not found in graph")?;

        let mut visited = HashSet::new();
        let mut descendants = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(*node_index);
        visited.insert(*node_index);

        while let Some(current_index) = queue.pop_front() {
            // Get all neighbors (outgoing edges)
            for neighbor_index in self.graph.neighbors_directed(current_index, Direction::Outgoing) {
                if !visited.contains(&neighbor_index) {
                    visited.insert(neighbor_index);
                    queue.push_back(neighbor_index);
                    
                    if let Some(neighbor_id) = self.graph.node_weight(neighbor_index) {
                        descendants.push(neighbor_id.clone());
                    }
                }
            }
        }

        debug!("Found {} descendants for node {}", descendants.len(), node_id);
        Ok(descendants)
    }

    /// Get all ancestors (upstream nodes) of a given node
    pub fn get_ancestors(&self, node_id: &str) -> Result<Vec<String>> {
        let node_index = self.node_map
            .get(node_id)
            .context("Node not found in graph")?;

        let mut visited = HashSet::new();
        let mut ancestors = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(*node_index);
        visited.insert(*node_index);

        while let Some(current_index) = queue.pop_front() {
            // Get all neighbors (incoming edges)
            for neighbor_index in self.graph.neighbors_directed(current_index, Direction::Incoming) {
                if !visited.contains(&neighbor_index) {
                    visited.insert(neighbor_index);
                    queue.push_back(neighbor_index);
                    
                    if let Some(neighbor_id) = self.graph.node_weight(neighbor_index) {
                        ancestors.push(neighbor_id.clone());
                    }
                }
            }
        }

        debug!("Found {} ancestors for node {}", ancestors.len(), node_id);
        Ok(ancestors)
    }

    /// Get leaf nodes (nodes with no outgoing edges)
    pub fn get_leaf_nodes(&self) -> Result<Vec<String>> {
        let mut leaf_nodes = Vec::new();

        for (node_id, &node_index) in &self.node_map {
            let outgoing_edges = self.graph.neighbors_directed(node_index, Direction::Outgoing).count();
            if outgoing_edges == 0 {
                leaf_nodes.push(node_id.clone());
            }
        }

        Ok(leaf_nodes)
    }

    /// Get root nodes (nodes with no incoming edges)
    pub fn get_root_nodes(&self) -> Result<Vec<String>> {
        let mut root_nodes = Vec::new();

        for (node_id, &node_index) in &self.node_map {
            let incoming_edges = self.graph.neighbors_directed(node_index, Direction::Incoming).count();
            if incoming_edges == 0 {
                root_nodes.push(node_id.clone());
            }
        }

        Ok(root_nodes)
    }

    /// Calculate the maximum depth from a given node to any leaf node
    pub fn calculate_max_depth(&self, node_id: &str) -> Result<usize> {
        let node_index = self.node_map
            .get(node_id)
            .context("Node not found in graph")?;

        self.calculate_max_depth_recursive(*node_index, &mut HashSet::new())
    }

    /// Recursive helper for depth calculation
    fn calculate_max_depth_recursive(
        &self, 
        node_index: NodeIndex, 
        visited: &mut HashSet<NodeIndex>
    ) -> Result<usize> {
        if visited.contains(&node_index) {
            // Cycle detected, return 0 to avoid infinite recursion
            return Ok(0);
        }

        visited.insert(node_index);

        let mut max_depth = 0;
        let neighbors: Vec<_> = self.graph.neighbors_directed(node_index, Direction::Outgoing).collect();

        if neighbors.is_empty() {
            // Leaf node
            visited.remove(&node_index);
            return Ok(0);
        }

        for neighbor_index in neighbors {
            let depth = self.calculate_max_depth_recursive(neighbor_index, visited)?;
            max_depth = max_depth.max(depth + 1);
        }

        visited.remove(&node_index);
        Ok(max_depth)
    }

    /// Get direct dependencies (immediate upstream nodes) of a given node
    pub fn get_direct_dependencies(&self, node_id: &str) -> Result<Vec<String>> {
        let node_index = self.node_map
            .get(node_id)
            .context("Node not found in graph")?;

        let mut dependencies = Vec::new();
        for neighbor_index in self.graph.neighbors_directed(*node_index, Direction::Incoming) {
            if let Some(neighbor_id) = self.graph.node_weight(neighbor_index) {
                dependencies.push(neighbor_id.clone());
            }
        }

        Ok(dependencies)
    }

    /// Get direct dependents (immediate downstream nodes) of a given node
    pub fn get_direct_dependents(&self, node_id: &str) -> Result<Vec<String>> {
        let node_index = self.node_map
            .get(node_id)
            .context("Node not found in graph")?;

        let mut dependents = Vec::new();
        for neighbor_index in self.graph.neighbors_directed(*node_index, Direction::Outgoing) {
            if let Some(neighbor_id) = self.graph.node_weight(neighbor_index) {
                dependents.push(neighbor_id.clone());
            }
        }

        Ok(dependents)
    }

    /// Check if there's a path between two nodes
    pub fn has_path(&self, from_node: &str, to_node: &str) -> Result<bool> {
        let from_index = self.node_map
            .get(from_node)
            .context("From node not found in graph")?;
        
        let to_index = self.node_map
            .get(to_node)
            .context("To node not found in graph")?;

        // BFS to find path
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(*from_index);
        visited.insert(*from_index);

        while let Some(current_index) = queue.pop_front() {
            if current_index == *to_index {
                return Ok(true);
            }

            for neighbor_index in self.graph.neighbors_directed(current_index, Direction::Outgoing) {
                if !visited.contains(&neighbor_index) {
                    visited.insert(neighbor_index);
                    queue.push_back(neighbor_index);
                }
            }
        }

        Ok(false)
    }

    /// Get the shortest path between two nodes
    pub fn get_shortest_path(&self, from_node: &str, to_node: &str) -> Result<Option<Vec<String>>> {
        let from_index = self.node_map
            .get(from_node)
            .context("From node not found in graph")?;
        
        let to_index = self.node_map
            .get(to_node)
            .context("To node not found in graph")?;

        // BFS to find shortest path
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        queue.push_back(*from_index);
        visited.insert(*from_index);

        while let Some(current_index) = queue.pop_front() {
            if current_index == *to_index {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = *to_index;
                
                // Build path in reverse
                while let Some(&prev) = parent.get(&current) {
                    if let Some(node_id) = self.graph.node_weight(current) {
                        path.push(node_id.clone());
                    }
                    current = prev;
                }
                
                // Add starting node
                if let Some(start_node_id) = self.graph.node_weight(*from_index) {
                    path.push(start_node_id.clone());
                }
                
                path.reverse();
                return Ok(Some(path));
            }

            for neighbor_index in self.graph.neighbors_directed(current_index, Direction::Outgoing) {
                if !visited.contains(&neighbor_index) {
                    visited.insert(neighbor_index);
                    parent.insert(neighbor_index, current_index);
                    queue.push_back(neighbor_index);
                }
            }
        }

        Ok(None)
    }

    /// Get graph statistics
    pub fn get_statistics(&self) -> GraphStatistics {
        let total_nodes = self.node_count();
        let total_edges = self.edge_count();
        
        let leaf_nodes = self.get_leaf_nodes().unwrap_or_default().len();
        let root_nodes = self.get_root_nodes().unwrap_or_default().len();
        
        // Calculate average degree
        let mut total_degree = 0;
        for &node_index in self.node_map.values() {
            let in_degree = self.graph.neighbors_directed(node_index, Direction::Incoming).count();
            let out_degree = self.graph.neighbors_directed(node_index, Direction::Outgoing).count();
            total_degree += in_degree + out_degree;
        }
        
        let average_degree = if total_nodes > 0 {
            total_degree as f64 / total_nodes as f64
        } else {
            0.0
        };

        GraphStatistics {
            total_nodes,
            total_edges,
            leaf_nodes,
            root_nodes,
            average_degree,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GraphStatistics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub leaf_nodes: usize,
    pub root_nodes: usize,
    pub average_degree: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> LineageGraph {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add nodes: A -> B -> C
        //            A -> D -> C
        let a = graph.add_node("A".to_string());
        let b = graph.add_node("B".to_string());
        let c = graph.add_node("C".to_string());
        let d = graph.add_node("D".to_string());

        node_map.insert("A".to_string(), a);
        node_map.insert("B".to_string(), b);
        node_map.insert("C".to_string(), c);
        node_map.insert("D".to_string(), d);

        // Add edges
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(a, d, ());
        graph.add_edge(d, c, ());

        LineageGraph::new(graph, node_map)
    }

    #[test]
    fn test_get_descendants() {
        let graph = create_test_graph();
        let descendants = graph.get_descendants("A").unwrap();
        
        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&"B".to_string()));
        assert!(descendants.contains(&"C".to_string()));
        assert!(descendants.contains(&"D".to_string()));
    }

    #[test]
    fn test_get_ancestors() {
        let graph = create_test_graph();
        let ancestors = graph.get_ancestors("C").unwrap();
        
        assert_eq!(ancestors.len(), 3);
        assert!(ancestors.contains(&"A".to_string()));
        assert!(ancestors.contains(&"B".to_string()));
        assert!(ancestors.contains(&"D".to_string()));
    }

    #[test]
    fn test_get_leaf_nodes() {
        let graph = create_test_graph();
        let leaf_nodes = graph.get_leaf_nodes().unwrap();
        
        assert_eq!(leaf_nodes.len(), 1);
        assert!(leaf_nodes.contains(&"C".to_string()));
    }

    #[test]
    fn test_get_root_nodes() {
        let graph = create_test_graph();
        let root_nodes = graph.get_root_nodes().unwrap();
        
        assert_eq!(root_nodes.len(), 1);
        assert!(root_nodes.contains(&"A".to_string()));
    }

    #[test]
    fn test_has_path() {
        let graph = create_test_graph();
        
        assert!(graph.has_path("A", "C").unwrap());
        assert!(graph.has_path("B", "C").unwrap());
        assert!(!graph.has_path("C", "A").unwrap());
        assert!(!graph.has_path("B", "D").unwrap());
    }

    #[test]
    fn test_calculate_max_depth() {
        let graph = create_test_graph();
        
        assert_eq!(graph.calculate_max_depth("A").unwrap(), 2);
        assert_eq!(graph.calculate_max_depth("B").unwrap(), 1);
        assert_eq!(graph.calculate_max_depth("D").unwrap(), 1);
        assert_eq!(graph.calculate_max_depth("C").unwrap(), 0);
    }
}