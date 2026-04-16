use crate::optimizer::abstractions::{Graph, CostMetric};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MockNode(pub String);

#[derive(Debug, Clone)]
pub struct MockEdge {
    pub weight: f64,
}

pub struct MockGraph {
    pub nodes: HashSet<MockNode>,
    pub adjacency: HashMap<MockNode, Vec<(MockNode, MockEdge)>>,
    pub in_degrees: HashMap<MockNode, usize>,
    pub out_degrees: HashMap<MockNode, usize>,
}

impl MockGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            adjacency: HashMap::new(),
            in_degrees: HashMap::new(),
            out_degrees: HashMap::new(),
        }
    }
}

impl Graph for MockGraph {
    type Node = MockNode;
    type Edge = MockEdge;

    fn nodes(&self) -> Vec<Self::Node> {
        self.nodes.iter().cloned().collect()
    }

    fn edges_from(&self, n: &Self::Node) -> Vec<(Self::Node, Self::Edge)> {
        self.adjacency.get(n).cloned().unwrap_or_default()
    }

    fn in_degree(&self, n: &Self::Node) -> usize {
        *self.in_degrees.get(n).unwrap_or(&0)
    }

    fn out_degree(&self, n: &Self::Node) -> usize {
        *self.out_degrees.get(n).unwrap_or(&0)
    }

    fn add_edge(&mut self, from: Self::Node, to: Self::Node, edge: Self::Edge) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.adjacency.entry(from.clone()).or_default().push((to.clone(), edge));
        *self.out_degrees.entry(from).or_insert(0) += 1;
        *self.in_degrees.entry(to).or_insert(0) += 1;
    }

    fn remove_edge(&mut self, from: &Self::Node, to: &Self::Node) {
        if let Some(edges) = self.adjacency.get_mut(from) {
            if let Some(pos) = edges.iter().position(|(t, _)| t == to) {
                edges.remove(pos);
                *self.out_degrees.get_mut(from).unwrap() -= 1;
                *self.in_degrees.get_mut(to).unwrap() -= 1;
            }
        }
    }
}

pub struct DefaultCostMetric;

impl CostMetric<MockEdge> for DefaultCostMetric {
    fn cost(&self, edge: &MockEdge) -> f64 {
        edge.weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_graph_basic() {
        let mut g = MockGraph::new();
        let n1 = MockNode("A".to_string());
        let n2 = MockNode("B".to_string());
        g.add_edge(n1.clone(), n2.clone(), MockEdge { weight: 1.0 });

        assert!(g.nodes().contains(&n1));
        assert_eq!(g.out_degree(&n1), 1);
        assert_eq!(g.in_degree(&n2), 1);
    }

    #[test]
    fn test_degree_integrity_and_removal() {
        let mut g = MockGraph::new();
        let n1 = MockNode("A".to_string());
        let n2 = MockNode("B".to_string());
        
        g.add_edge(n1.clone(), n2.clone(), MockEdge { weight: 1.0 });
        assert_eq!(g.out_degree(&n1), 1);
        assert_eq!(g.in_degree(&n2), 1);
        
        g.remove_edge(&n1, &n2);
        assert_eq!(g.out_degree(&n1), 0);
        assert_eq!(g.in_degree(&n2), 0);
    }

    #[test]
    fn test_remove_nonexistent_edge() {
        let mut g = MockGraph::new();
        let n1 = MockNode("A".to_string());
        let n2 = MockNode("B".to_string());
        
        // Ensure no panic on removing non-existent
        g.remove_edge(&n1, &n2);
        assert_eq!(g.out_degree(&n1), 0);
    }

    #[test]
    fn test_multigraph_support() {
        let mut g = MockGraph::new();
        let n1 = MockNode("A".to_string());
        let n2 = MockNode("B".to_string());
        
        g.add_edge(n1.clone(), n2.clone(), MockEdge { weight: 1.0 });
        g.add_edge(n1.clone(), n2.clone(), MockEdge { weight: 2.0 });
        
        assert_eq!(g.out_degree(&n1), 2);
        assert_eq!(g.edges_from(&n1).len(), 2);
        
        g.remove_edge(&n1, &n2);
        assert_eq!(g.out_degree(&n1), 1);
        assert_eq!(g.edges_from(&n1).len(), 1);
    }

    #[test]
    fn test_node_deduplication() {
        let mut g = MockGraph::new();
        let n1 = MockNode("A".to_string());
        let n2 = MockNode("B".to_string());
        
        g.add_edge(n1.clone(), n2.clone(), MockEdge { weight: 1.0 });
        g.add_edge(n2.clone(), n1.clone(), MockEdge { weight: 1.0 });
        
        let nodes = g.nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_cost_metric_decoupling() {
        let metric = DefaultCostMetric;
        let edge = MockEdge { weight: 42.0 };
        assert_eq!(metric.cost(&edge), 42.0);
    }
}
