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

    // Verify the circuit is closed. In a non-Eulerian graph it is possible for
    // all edges to be consumed (used.len() == total_edges) while still producing
    // an open path rather than a closed circuit — for example A→B, A→C yields
    // the traversal [A,C,B] which uses both edges but never returns to A.
    if circuit.last() != Some(&start) {
        anyhow::bail!(
            "Hierholzer: circuit does not return to start node — graph is not Eulerian"
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

    // --- Backtracking edge cases ---

    #[test]
    fn test_single_node_no_edges() {
        // Isolated node: total_edges == 0, early-return path.
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit, vec![a]);
    }

    #[test]
    fn test_self_loop() {
        // A → A: the minimal non-trivial Eulerian circuit.
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        g.add_edge(a, a, 1.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 2);
        assert_eq!(circuit[0], a);
        assert_eq!(circuit[1], a);
    }

    #[test]
    fn test_multiple_self_loops() {
        // A → A twice: two self-loops at the same node.
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        g.add_edge(a, a, 1.0);
        g.add_edge(a, a, 2.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 3); // both loops traversed
        assert_eq!(circuit[0], a);
        assert_eq!(circuit[2], a);
    }

    #[test]
    fn test_parallel_edges() {
        // Two edges A→B and two edges B→A (multi-graph).
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        g.add_edge(a, b, 1.0);
        g.add_edge(a, b, 2.0);
        g.add_edge(b, a, 1.0);
        g.add_edge(b, a, 2.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 5); // 4 edges + return
        assert_eq!(circuit.first(), circuit.last());
        assert_eq!(*circuit.first().unwrap(), a);
    }

    #[test]
    fn test_forced_backtrack_splice() {
        // A→B, B→C→A (outer loop) and B→D→B (inner hanging loop).
        //
        // Depending on edge order, the algorithm may follow A→B→C→A,
        // exhaust A with no remaining edges, pop back to C then B,
        // and only then splice in the B→D→B sub-circuit. This exercises
        // the core Hierholzer backtrack: a node that appeared on the stack
        // earlier still has unused edges and they get incorporated via the
        // shared adjacency deque.
        //
        // Only valid full circuit: A→B→?→B→?→A (both B-exits must be used).
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        let d = g.add_node("D");
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, a, 1.0);
        g.add_edge(b, d, 1.0);
        g.add_edge(d, b, 1.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 6); // 5 edges + return
        assert_eq!(*circuit.first().unwrap(), a);
        assert_eq!(*circuit.last().unwrap(), a);
        assert!(circuit.contains(&d), "D must be visited via the B→D→B sub-circuit");
    }

    #[test]
    fn test_three_loops_at_hub() {
        // Three independent loops all anchored at A:
        //   A→B→A, A→C→A, A→D→A
        // The algorithm must backtrack and splice twice to cover all loops.
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        let d = g.add_node("D");
        g.add_edge(a, b, 1.0);
        g.add_edge(b, a, 1.0);
        g.add_edge(a, c, 1.0);
        g.add_edge(c, a, 1.0);
        g.add_edge(a, d, 1.0);
        g.add_edge(d, a, 1.0);

        let circuit = directed_eulerian_circuit(&g, a).unwrap();
        assert_eq!(circuit.len(), 7); // 6 edges + return
        assert_eq!(*circuit.first().unwrap(), a);
        assert_eq!(*circuit.last().unwrap(), a);
        assert!(circuit.contains(&b));
        assert!(circuit.contains(&c));
        assert!(circuit.contains(&d));
    }

    #[test]
    fn test_disconnected_eulerian_fails() {
        // Two separate triangles that are individually Eulerian but not
        // reachable from each other. Starting at A cannot visit D,E,F.
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        let d = g.add_node("D");
        let e = g.add_node("E");
        let f = g.add_node("F");
        g.add_edge(a, b, 1.0);
        g.add_edge(b, c, 1.0);
        g.add_edge(c, a, 1.0);
        g.add_edge(d, e, 1.0);
        g.add_edge(e, f, 1.0);
        g.add_edge(f, d, 1.0);

        let result = directed_eulerian_circuit(&g, a);
        assert!(result.is_err(), "disconnected graph must fail");
    }

    #[test]
    fn test_start_with_no_outgoing_edges_fails() {
        // B→A exists but A has no outgoing edges.
        // Starting at A immediately exhausts the stack without using the edge.
        let mut g = DiGraph::<&str, f64>::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        g.add_edge(b, a, 1.0);

        let result = directed_eulerian_circuit(&g, a);
        assert!(result.is_err(), "start node with no outgoing edges must fail");
    }
}
