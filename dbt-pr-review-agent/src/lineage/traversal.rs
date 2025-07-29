use crate::lineage::graph::LineageGraph;
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};

/// Graph traversal utilities for lineage analysis
pub struct GraphTraversal;

impl GraphTraversal {
    /// Perform breadth-first search to find all reachable nodes
    pub fn bfs_reachable(
        graph: &LineageGraph,
        start_nodes: &[String],
        direction: TraversalDirection,
    ) -> Result<HashSet<String>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Initialize queue with start nodes
        for node in start_nodes {
            queue.push_back(node.clone());
            visited.insert(node.clone());
        }

        while let Some(current_node) = queue.pop_front() {
            let neighbors = match direction {
                TraversalDirection::Downstream => graph.get_direct_dependents(&current_node)?,
                TraversalDirection::Upstream => graph.get_direct_dependencies(&current_node)?,
            };

            for neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(visited)
    }

    /// Perform depth-first search with visit tracking
    pub fn dfs_with_path(
        graph: &LineageGraph,
        start_node: &str,
        target_node: &str,
        direction: TraversalDirection,
    ) -> Result<Option<Vec<String>>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if Self::dfs_recursive(graph, start_node, target_node, direction, &mut visited, &mut path)? {
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    /// Recursive DFS helper
    fn dfs_recursive(
        graph: &LineageGraph,
        current_node: &str,
        target_node: &str,
        direction: TraversalDirection,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<bool> {
        visited.insert(current_node.to_string());
        path.push(current_node.to_string());

        if current_node == target_node {
            return Ok(true);
        }

        let neighbors = match direction {
            TraversalDirection::Downstream => graph.get_direct_dependents(current_node)?,
            TraversalDirection::Upstream => graph.get_direct_dependencies(current_node)?,
        };

        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if Self::dfs_recursive(graph, &neighbor, target_node, direction, visited, path)? {
                    return Ok(true);
                }
            }
        }

        path.pop();
        Ok(false)
    }

    /// Find strongly connected components (for cycle detection)
    pub fn find_cycles(graph: &LineageGraph) -> Result<Vec<Vec<String>>> {
        // TODO: Implement Tarjan's algorithm for finding SCCs
        // This is a placeholder implementation
        Ok(Vec::new())
    }

    /// Calculate impact radius from a set of changed nodes
    pub fn calculate_impact_radius(
        graph: &LineageGraph,
        changed_nodes: &[String],
        max_depth: Option<usize>,
    ) -> Result<HashMap<String, usize>> {
        let mut impact_radius = HashMap::new();
        let max_depth = max_depth.unwrap_or(usize::MAX);

        for node in changed_nodes {
            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            
            queue.push_back((node.clone(), 0));
            visited.insert(node.clone());

            while let Some((current_node, depth)) = queue.pop_front() {
                if depth >= max_depth {
                    continue;
                }

                let dependents = graph.get_direct_dependents(&current_node)?;
                for dependent in dependents {
                    if !visited.contains(&dependent) {
                        visited.insert(dependent.clone());
                        queue.push_back((dependent.clone(), depth + 1));
                        
                        // Track the minimum depth at which each node is reached
                        impact_radius
                            .entry(dependent)
                            .and_modify(|d| *d = (*d).min(depth + 1))
                            .or_insert(depth + 1);
                    }
                }
            }
        }

        Ok(impact_radius)
    }

    /// Find critical path (longest path) through the dependency graph
    pub fn find_critical_path(
        graph: &LineageGraph,
        start_node: &str,
    ) -> Result<(Vec<String>, usize)> {
        let mut max_path = Vec::new();
        let mut max_length = 0;
        let mut visited = HashSet::new();
        let mut current_path = Vec::new();

        Self::find_longest_path_recursive(
            graph,
            start_node,
            &mut visited,
            &mut current_path,
            &mut max_path,
            &mut max_length,
        )?;

        Ok((max_path, max_length))
    }

    /// Recursive helper for finding longest path
    fn find_longest_path_recursive(
        graph: &LineageGraph,
        current_node: &str,
        visited: &mut HashSet<String>,
        current_path: &mut Vec<String>,
        max_path: &mut Vec<String>,
        max_length: &mut usize,
    ) -> Result<()> {
        visited.insert(current_node.to_string());
        current_path.push(current_node.to_string());

        let dependents = graph.get_direct_dependents(current_node)?;
        let mut has_unvisited_dependents = false;

        for dependent in dependents {
            if !visited.contains(&dependent) {
                has_unvisited_dependents = true;
                Self::find_longest_path_recursive(
                    graph,
                    &dependent,
                    visited,
                    current_path,
                    max_path,
                    max_length,
                )?;
            }
        }

        // If this is a leaf node or all dependents are visited
        if !has_unvisited_dependents && current_path.len() > *max_length {
            *max_length = current_path.len();
            max_path.clear();
            max_path.extend_from_slice(current_path);
        }

        current_path.pop();
        visited.remove(current_node);
        Ok(())
    }

    /// Get subgraph containing only specified nodes and their connections
    pub fn extract_subgraph(
        graph: &LineageGraph,
        nodes: &[String],
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut subgraph = HashMap::new();
        let node_set: HashSet<String> = nodes.iter().cloned().collect();

        for node in nodes {
            let dependents = graph.get_direct_dependents(node)?;
            let filtered_dependents: Vec<String> = dependents
                .into_iter()
                .filter(|dep| node_set.contains(dep))
                .collect();
            
            subgraph.insert(node.clone(), filtered_dependents);
        }

        Ok(subgraph)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TraversalDirection {
    Upstream,
    Downstream,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lineage::graph::LineageGraph;
    use petgraph::Graph;
    use std::collections::HashMap;

    fn create_test_graph() -> LineageGraph {
        let mut graph = Graph::new();
        let mut node_map = HashMap::new();

        // Create a simple DAG: A -> B -> C, A -> D -> C
        let a = graph.add_node("A".to_string());
        let b = graph.add_node("B".to_string());
        let c = graph.add_node("C".to_string());
        let d = graph.add_node("D".to_string());

        node_map.insert("A".to_string(), a);
        node_map.insert("B".to_string(), b);
        node_map.insert("C".to_string(), c);
        node_map.insert("D".to_string(), d);

        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(a, d, ());
        graph.add_edge(d, c, ());

        LineageGraph::new(graph, node_map)
    }

    #[test]
    fn test_bfs_reachable_downstream() {
        let graph = create_test_graph();
        let reachable = GraphTraversal::bfs_reachable(
            &graph,
            &["A".to_string()],
            TraversalDirection::Downstream,
        ).unwrap();

        assert_eq!(reachable.len(), 4); // A, B, C, D
        assert!(reachable.contains("A"));
        assert!(reachable.contains("B"));
        assert!(reachable.contains("C"));
        assert!(reachable.contains("D"));
    }

    #[test]
    fn test_bfs_reachable_upstream() {
        let graph = create_test_graph();
        let reachable = GraphTraversal::bfs_reachable(
            &graph,
            &["C".to_string()],
            TraversalDirection::Upstream,
        ).unwrap();

        assert_eq!(reachable.len(), 4); // A, B, C, D
        assert!(reachable.contains("A"));
        assert!(reachable.contains("B"));
        assert!(reachable.contains("C"));
        assert!(reachable.contains("D"));
    }

    #[test]
    fn test_impact_radius() {
        let graph = create_test_graph();
        let impact = GraphTraversal::calculate_impact_radius(
            &graph,
            &["A".to_string()],
            Some(2),
        ).unwrap();

        // B and D should be at depth 1, C should be at depth 2
        assert_eq!(impact.get("B"), Some(&1));
        assert_eq!(impact.get("D"), Some(&1));
        assert_eq!(impact.get("C"), Some(&2));
    }

    #[test]
    fn test_extract_subgraph() {
        let graph = create_test_graph();
        let subgraph = GraphTraversal::extract_subgraph(
            &graph,
            &["A".to_string(), "B".to_string(), "C".to_string()],
        ).unwrap();

        // Should only include connections between A, B, C
        assert_eq!(subgraph.get("A").unwrap(), &vec!["B".to_string()]);
        assert_eq!(subgraph.get("B").unwrap(), &vec!["C".to_string()]);
        assert_eq!(subgraph.get("C").unwrap(), &Vec::<String>::new());
    }
}