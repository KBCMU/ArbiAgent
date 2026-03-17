/// Optimal bipartite assignment via the Kuhn-Munkres (Hungarian) algorithm.
///
/// Given an `n_rows × n_cols` weight matrix, returns the maximum-weight
/// one-to-one assignment.  Runs in O(n² · m) where n = min(rows, cols)
/// and m = max(rows, cols), which is fine for our bucket sizes (< 30).

/// Find the maximum-weight bipartite assignment.
///
/// `weights` is indexed as `weights[row][col]`.
/// Returns `(row, col, weight)` triples for each assigned pair, excluding
/// any pair whose weight is below `min_weight`.
pub fn max_weight_assignment(
    weights: &[Vec<f64>],
    min_weight: f64,
) -> Vec<(usize, usize, f64)> {
    let n_rows = weights.len();
    if n_rows == 0 {
        return vec![];
    }
    let n_cols = weights[0].len();
    if n_cols == 0 {
        return vec![];
    }

    // The classic algorithm works on square matrices with minimum cost.
    // We convert: pad to square, negate weights for min-cost, then run.
    let n = n_rows.max(n_cols);

    // Find the maximum weight to compute costs = max_w - w
    let max_w = weights
        .iter()
        .flat_map(|row| row.iter())
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // Build square cost matrix: cost = max_w - weight (so minimizing cost = maximizing weight)
    // Padded entries get cost 0 (= max_w - max_w equivalent, they're "free" dummy assignments)
    let mut cost = vec![vec![0.0_f64; n]; n];
    for i in 0..n_rows {
        for j in 0..n_cols {
            cost[i][j] = max_w - weights[i][j];
        }
    }

    let assignment = kuhn_munkres_min(&cost, n);

    // Collect results, filtering out padding and below-threshold
    let mut result = Vec::new();
    for (row, &col) in assignment.iter().enumerate() {
        if row < n_rows && col < n_cols {
            let w = weights[row][col];
            if w >= min_weight {
                result.push((row, col, w));
            }
        }
    }
    result
}

/// Classic O(n³) Kuhn-Munkres for minimum-cost assignment on an n×n matrix.
/// Returns `assignment[row] = col` for each row.
fn kuhn_munkres_min(cost: &[Vec<f64>], n: usize) -> Vec<usize> {
    // u[i] = potential for row i, v[j] = potential for column j
    // p[j] = row assigned to column j (0 = unassigned, using 1-indexed rows internally)
    let mut u = vec![0.0_f64; n + 1];
    let mut v = vec![0.0_f64; n + 1];
    let mut p = vec![0_usize; n + 1]; // p[j] = row matched to col j (1-indexed)
    let mut way = vec![0_usize; n + 1]; // way[j] = previous col in alternating path

    for i in 1..=n {
        // Start augmenting from row i (1-indexed)
        p[0] = i;
        let mut j0 = 0_usize; // "virtual" column
        let mut min_v = vec![f64::INFINITY; n + 1];
        let mut used = vec![false; n + 1];

        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = f64::INFINITY;
            let mut j1 = 0_usize;

            for j in 1..=n {
                if used[j] {
                    continue;
                }
                let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                if cur < min_v[j] {
                    min_v[j] = cur;
                    way[j] = j0;
                }
                if min_v[j] < delta {
                    delta = min_v[j];
                    j1 = j;
                }
            }

            // Update potentials
            for j in 0..=n {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    min_v[j] -= delta;
                }
            }

            j0 = j1;

            if p[j0] == 0 {
                break; // Found an augmenting path
            }
        }

        // Trace the augmenting path back and update assignments
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }

    // Convert from 1-indexed to 0-indexed: assignment[row] = col
    let mut assignment = vec![0_usize; n];
    for j in 1..=n {
        if p[j] > 0 {
            assignment[p[j] - 1] = j - 1;
        }
    }
    assignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_2x2() {
        let weights = vec![
            vec![10.0, 5.0],
            vec![5.0, 10.0],
        ];
        let result = max_weight_assignment(&weights, 0.0);
        assert_eq!(result.len(), 2);
        let total: f64 = result.iter().map(|(_, _, w)| w).sum();
        assert!((total - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_greedy_would_fail() {
        // Greedy picks (0,0)=9, then (1,1)=1 → total 10
        // Optimal is (0,1)=8, (1,0)=8 → total 16
        let weights = vec![
            vec![9.0, 8.0],
            vec![8.0, 1.0],
        ];
        let result = max_weight_assignment(&weights, 0.0);
        let total: f64 = result.iter().map(|(_, _, w)| w).sum();
        assert!((total - 16.0).abs() < 1e-9, "total was {}", total);
    }

    #[test]
    fn test_min_weight_filter() {
        let weights = vec![
            vec![10.0, 2.0],
            vec![2.0, 10.0],
        ];
        let result = max_weight_assignment(&weights, 5.0);
        assert_eq!(result.len(), 2);

        // All below threshold
        let weights2 = vec![
            vec![1.0, 2.0],
            vec![2.0, 1.0],
        ];
        let result2 = max_weight_assignment(&weights2, 5.0);
        assert_eq!(result2.len(), 0);
    }

    #[test]
    fn test_rectangular_more_cols() {
        let weights = vec![
            vec![10.0, 5.0, 8.0],
            vec![3.0, 9.0, 4.0],
        ];
        let result = max_weight_assignment(&weights, 0.0);
        assert_eq!(result.len(), 2);
        let total: f64 = result.iter().map(|(_, _, w)| w).sum();
        assert!((total - 19.0).abs() < 1e-9, "total was {}", total);
    }

    #[test]
    fn test_rectangular_more_rows() {
        let weights = vec![
            vec![10.0, 3.0],
            vec![5.0, 9.0],
            vec![8.0, 4.0],
        ];
        let result = max_weight_assignment(&weights, 0.0);
        assert_eq!(result.len(), 2);
        let total: f64 = result.iter().map(|(_, _, w)| w).sum();
        assert!((total - 19.0).abs() < 1e-9, "total was {}", total);
    }

    #[test]
    fn test_empty() {
        let result = max_weight_assignment(&[], 0.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_pair() {
        let weights = vec![vec![42.0]];
        let result = max_weight_assignment(&weights, 0.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (0, 0, 42.0));
    }
}
