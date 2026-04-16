//! Graph Abstractions for Algorithm Interchangeability
//!
//! This module provides abstract traits for graph operations and algorithms,
//! enabling interchangeable implementations of graph structures, Eulerian
//! circuit finding, and Chinese Postman Problem optimization.

use anyhow::Result;
use std::collections::HashSet;
use std::hash::Hash;
use crate::geo::types::Coordinate;

/// Abstract graph for algorithm interchangeability.
///
/// This trait defines the minimum interface required for graph algorithms
/// to operate on arbitrary graph implementations.
pub trait Graph {
    /// The node type used in this graph
    type Node: Eq + Hash + Clone;
    
    /// The edge type used in this graph
    type Edge: Clone;

    /// Returns a slice of all nodes in the graph
    fn nodes(&self) -> &[Self::Node];

    /// Returns all outgoing edges from a given node as pairs of (target_node, edge)
    fn edges_from(&self, n: &Self::Node) -> Vec<(Self::Node, Self::Edge)>;

    /// Returns the in-degree (number of incoming edges) for a given node
    fn in_degree(&self, n: &Self::Node) -> usize;

    /// Returns the out-degree (number of outgoing edges) for a given node
    fn out_degree(&self, n: &Self::Node) -> usize;

    /// Adds an edge between two nodes.
    fn add_edge(&mut self, from: Self::Node, to: Self::Node, edge: Self::Edge);

    /// Removes an edge between two nodes.
    fn remove_edge(&mut self, from: &Self::Node, to: &Self::Node);
}

/// Interface for accessing spatial data for graph nodes.
///
/// Decouples algorithms from direct coordinate storage.
pub trait SpatialProvider {
    /// The node type used in the graph
    type Node: Eq + Hash + Clone;

    /// Get the coordinate for a given node
    fn get_coordinate(&self, node: &Self::Node) -> Option<Coordinate>;
    
    /// Calculate distance between two nodes
    fn distance(&self, from: &Self::Node, to: &Self::Node) -> Option<f64>;
}

/// Defines how edge costs are calculated.
///
/// Decouples algorithms from specific spatial or physical distance calculations.
pub trait CostMetric<E> {
    fn cost(&self, edge: &E) -> f64;
}

/// Algorithm for finding Eulerian circuits in a graph.
///
/// An Eulerian circuit visits every edge exactly once and returns to the starting node.
/// This trait abstracts over different implementations (Hierholzer's, Fleury's, etc.)
/// and different graph backends (petgraph, custom, etc.).
///
/// # Type Parameters
/// - `Graph`: The graph type this algorithm operates on
pub trait EulerianAlgorithm {
    /// The type of graph this algorithm can process
    type Graph: Graph;

    /// Find an Eulerian circuit starting from the given node.
    ///
    /// # Arguments
    /// * `g` - The graph to search for a circuit
    /// * `start` - The starting node for the circuit
    ///
    /// # Returns
    /// * `Ok(Vec<Node>)` - The sequence of nodes in the Eulerian circuit
    /// * `Err` - If no circuit exists (graph is not Eulerian or not connected)
    fn find_circuit(&self, g: &Self::Graph, start: &<Self::Graph as Graph>::Node) -> Result<Vec<<Self::Graph as Graph>::Node>>;
}

/// Chinese Postman Problem (CPP) Optimizer.
///
/// The CPP finds the shortest closed path that traverses every edge of a graph
/// at least once. For Eulerian graphs, this is simply an Eulerian circuit.
/// For non-Eulerian graphs, edges must be duplicated (balanced) to make the
/// graph Eulerian, and the optimizer finds the minimum-cost set of edges to add.
///
/// # Type Parameters
/// - `Graph`: The graph type this optimizer operates on
pub trait CPPOptimizer {
    /// The type of graph this optimizer can balance
    type Graph: Graph;

    /// Balance the graph to make it Eulerian by adding minimum-cost edges.
    ///
    /// This method modifies the graph in-place by adding edges to balance
    /// the in-degree and out-degree of each node.
    ///
    /// # Arguments
    /// * `g` - The graph to balance
    ///
    /// # Returns
    /// * `Ok(f64)` - The total cost of the added edges
    /// * `Err` - If balancing is not possible
    fn balance(&self, g: &mut Self::Graph) -> Result<f64>;
}

/// Helper trait for graphs that can provide node references from indices
/// Useful for bridging between trait objects and concrete implementations
pub trait NodeLookup {
    type Node;
    type Index;
    
    /// Get a node reference from an index
    fn node_from_index(&self, idx: Self::Index) -> Option<&Self::Node>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Simple test graph implementation
    #[derive(Debug, Clone)]
    struct SimpleNode {
        id: String,
    }
    
    impl Eq for SimpleNode {}
    impl PartialEq for SimpleNode {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl Hash for SimpleNode {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.id.hash(state);
        }
    }

    type SimpleEdge = f64;

    struct SimpleGraph {
        nodes: Vec<SimpleNode>,
        adjacency: HashMap<String, Vec<(String, SimpleEdge)>>,
        in_degrees: HashMap<String, usize>,
        out_degrees: HashMap<String, usize>,
    }

    impl SimpleGraph {
        fn new() -> Self {
            Self {
                nodes: Vec::new(),
                adjacency: HashMap::new(),
                in_degrees: HashMap::new(),
                out_degrees: HashMap::new(),
            }
        }

        fn add_node(&mut self, id: &str) -> &mut Self {
            self.nodes.push(SimpleNode { id: id.to_string() });
            self.in_degrees.entry(id.to_string()).or_insert(0);
            self.out_degrees.entry(id.to_string()).or_insert(0);
            self
        }

        fn add_edge(&mut self, from: &str, to: &str, weight: f64) -> &mut Self {
            self.adjacency
                .entry(from.to_string())
                .or_default()
                .push((to.to_string(), weight));
            *self.out_degrees.entry(from.to_string()).or_insert(0) += 1;
            *self.in_degrees.entry(to.to_string()).or_insert(0) += 1;
            self
        }
    }

    impl Graph for SimpleGraph {
        type Node = SimpleNode;
        type Edge = SimpleEdge;

        fn nodes(&self) -> &[Self::Node] {
            &self.nodes
        }

        fn edges_from(&self, n: &Self::Node) -> Vec<(Self::Node, Self::Edge)> {
            self.adjacency
                .get(&n.id)
                .map(|edges| {
                    edges
                        .iter()
                        .map(|(to, weight)| (SimpleNode { id: to.clone() }, *weight))
                        .collect()
                })
                .unwrap_or_default()
        }

        fn in_degree(&self, n: &Self::Node) -> usize {
            *self.in_degrees.get(&n.id).unwrap_or(&0)
        }

        fn out_degree(&self, n: &Self::Node) -> usize {
            *self.out_degrees.get(&n.id).unwrap_or(&0)
        }

        fn add_edge(&mut self, from: Self::Node, to: Self::Node, edge: Self::Edge) {
            self.adjacency
                .entry(from.id.clone())
                .or_default()
                .push((to.id.clone(), edge));
            *self.out_degrees.entry(from.id).or_insert(0) += 1;
            *self.in_degrees.entry(to.id).or_insert(0) += 1;
        }

        fn remove_edge(&mut self, from: &Self::Node, to: &Self::Node) {
            if let Some(edges) = self.adjacency.get_mut(&from.id) {
                if let Some(pos) = edges.iter().position(|(t, _)| t == &to.id) {
                    edges.remove(pos);
                    *self.out_degrees.entry(from.id.clone()).or_insert(1) -= 1;
                    *self.in_degrees.entry(to.id.clone()).or_insert(1) -= 1;
                }
            }
        }
    }

    #[test]
    fn test_graph_trait_simple() {
        let mut g = SimpleGraph::new();
        g.add_node("A");
        g.add_node("B");
        g.add_node("C");
        g.add_edge("A", "B", 1.0);
        g.add_edge("B", "C", 2.0);
        g.add_edge("C", "A", 3.0);

        assert_eq!(g.nodes().len(), 3);

        let node_a = &g.nodes()[0];
        assert_eq!(node_a.id, "A");
        assert_eq!(g.out_degree(node_a), 1);
        assert_eq!(g.in_degree(node_a), 1);

        let edges = g.edges_from(node_a);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0.id, "B");
        assert_eq!(edges[0].1, 1.0);
    }

    #[test]
    fn test_graph_degrees() {
        let mut g = SimpleGraph::new();
        g.add_node("A");
        g.add_node("B");
        g.add_edge("A", "B", 1.0);
        g.add_edge("B", "A", 2.0);
        g.add_edge("A", "A", 3.0); // self-loop

        let node_a = &g.nodes()[0];
        let node_b = &g.nodes()[1];

        assert_eq!(g.out_degree(node_a), 2);
        assert_eq!(g.in_degree(node_a), 2);
        assert_eq!(g.out_degree(node_b), 1);
        assert_eq!(g.in_degree(node_b), 1);
    }
}
