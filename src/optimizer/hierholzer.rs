use anyhow::Result;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};

/// Iterative Hierholzer's algorithm for directed Eulerian circuits.
///
/// Ported from /home/drone/rmp.ca/backend/app/hierholzer.py
///
/// The algorithm:
/// 1. Start at a node, follow unused edges until stuck (must return to start).
/// 2. Find a node on the current circuit that still has unused edges.
/// 3. Start a sub-tour from that node, splice it into the circuit.
/// 4. Repeat until all edges are used.
pub fn directed_eulerian_circuit<N, E>(
    graph: &DiGraph<N, E>,
    start: NodeIndex,
) -> Result<Vec<NodeIndex>> {
    let total_edges = graph.edge_count();
    if total_edges == 0 {
        return Ok(vec![start]);
    }

    // Build mutable adjacency: node → deque of (target, edge_id)
    let mut adjacency: HashMap<NodeIndex, VecDeque<(NodeIndex, EdgeIndex)>> = HashMap::new();
    for node in graph.node_indices() {
        let edges: VecDeque<_> = graph
            .edges(node)
            .map(|e| (e.target(), e.id()))
            .collect();
        adjacency.insert(node, edges);
    }

    let mut used: HashSet<EdgeIndex> = HashSet::with_capacity(total_edges);
    let mut stack: Vec<NodeIndex> = vec![start];
    let mut circuit: Vec<NodeIndex> = Vec::with_capacity(total_edges + 1);

    while let Some(&current) = stack.last() {
        // Try to find an unused outgoing edge
        let next = loop {
            if let Some(adj) = adjacency.get_mut(&current) {
                if let Some((target, edge_id)) = adj.pop_front() {
                    if used.insert(edge_id) {
                        // Successfully used this edge
                        break Some(target);
                    }
                    // Edge already used, try next
                } else {
                    break None; // No more edges
                }
            } else {
                break None;
            }
        };

        match next {
            Some(target) => {
                stack.push(target);
            }
            None => {
                // No unused edges from current — add to circuit
                circuit.push(stack.pop().unwrap());
            }
        }
    }

    circuit.reverse();

    // Verify all edges were used
    if used.len() != total_edges {
        anyhow::bail!(
            "Hierholzer: only used {}/{} edges — graph may not be Eulerian or connected",
            used.len(),
            total_edges
        );
    }

    Ok(circuit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    #[test]
    fn test_simple_triangle() {
        // A → B → C → A
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, a, 1.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 4); // A B C A
        assert_eq!(circuit.first(), circuit.last());
    }

    #[test]
    fn test_figure_eight() {
        // A → B → A → C → A
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, 1.0);
        g.add_edge(b, a, 1.0);
        g.add_edge(a, c, 1.0);
        g.add_edge(c, a, 1.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 5); // visits all 4 edges + returns
        assert_eq!(circuit[0], a);
        assert_eq!(circuit[4], a);
    }

    #[test]
    fn test_non_eulerian_fails() {
        // A → B, A → C (no way back — not Eulerian)
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, 1.0);
        g.add_edge(a, c, 1.0);

        let result = directed_eulerian_circuit(&g, a);
        assert!(result.is_err());
    }
}
