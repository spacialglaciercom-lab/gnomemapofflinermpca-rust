use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};

/// Minimum-weight perfect matching for Eulerian balancing.
///
/// Given sets of supply (excess outgoing) and demand (excess incoming)
/// vertices with a cost matrix, find the minimum-cost assignment.
///
/// For small graphs (< 100 imbalanced vertices), use brute-force or
/// Hungarian algorithm. For larger graphs, use an approximation.
///
/// Port of optimize.py (_solve_cpp → scipy.optimize.linear_sum_assignment)
pub fn min_weight_matching(
    supply: &[NodeIndex],
    demand: &[NodeIndex],
    cost_matrix: &[Vec<f64>],
) -> Result<Vec<(NodeIndex, NodeIndex, f64)>> {
    anyhow::ensure!(
        supply.len() == demand.len(),
        "Supply ({}) and demand ({}) must be equal for perfect matching",
        supply.len(),
        demand.len()
    );

    if supply.is_empty() {
        return Ok(vec![]);
    }

    // TODO: Implement Hungarian algorithm for optimal matching
    // For now, use greedy nearest-neighbor as a placeholder
    let n = supply.len();
    let mut used_demand: Vec<bool> = vec![false; n];
    let mut result = Vec::with_capacity(n);

    for (i, &s) in supply.iter().enumerate() {
        let mut best_j = 0;
        let mut best_cost = f64::INFINITY;
        for (j, _) in demand.iter().enumerate() {
            if !used_demand[j] && cost_matrix[i][j] < best_cost {
                best_cost = cost_matrix[i][j];
                best_j = j;
            }
        }
        used_demand[best_j] = true;
        result.push((s, demand[best_j], best_cost));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_matching() {
        let supply: Vec<NodeIndex> = vec![];
        let demand: Vec<NodeIndex> = vec![];
        let cost_matrix: Vec<Vec<f64>> = vec![];

        let result = min_weight_matching(&supply, &demand, &cost_matrix).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_2x2_matching() {
        // Simple 2x2 case
        let supply: Vec<NodeIndex> = vec![NodeIndex::new(0), NodeIndex::new(1)];
        let demand: Vec<NodeIndex> = vec![NodeIndex::new(2), NodeIndex::new(3)];
        let cost_matrix = vec![
            vec![1.0, 4.0],
            vec![3.0, 2.0],
        ];

        let result = min_weight_matching(&supply, &demand, &cost_matrix).unwrap();
        assert_eq!(result.len(), 2);
        // Greedy should pick 0->2 (cost 1) then 1->3 (cost 2)
        // Total cost: 3.0
        let total_cost: f64 = result.iter().map(|(_, _, c)| c).sum();
        assert_eq!(total_cost, 3.0);
    }

    #[test]
    fn test_unequal_supply_demand_fails() {
        let supply: Vec<NodeIndex> = vec![NodeIndex::new(0)];
        let demand: Vec<NodeIndex> = vec![NodeIndex::new(1), NodeIndex::new(2)];
        let cost_matrix = vec![vec![1.0, 2.0]];

        let result = min_weight_matching(&supply, &demand, &cost_matrix);
        assert!(result.is_err());
    }
}
