//! Laser path optimization: closing, noise/island removal, merging,
//! simplification, ordering. See CLAUDE.md Section 8 and
//! `docs/adr/0004-geometry-boolean-ops.md`.
//!
//! Implemented so far: [`merge_adjacent_regions`] (boolean union via
//! `i_overlay`) and [`order_paths_by_travel_distance`]/[`score_path_order`]
//! (Phase 5, `docs/roadmap.md`). Path closing, noise/island removal,
//! and curve simplification are already handled upstream by
//! `laserprep_vectorize::VtracerVectorizer` (always-closed traced
//! paths; the "Minimum Area" parameter).

use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::simplify::SimplifyShape;
use laserprep_vectorize::{PolygonShape, VectorPath};

/// Merges touching or overlapping regions in `shapes` into as few
/// shapes as possible via a boolean self-union (CLAUDE.md Section 8:
/// "Merge Adjacent Regions"). Genuinely separate regions are returned
/// unchanged — this never increases the shape count, and holes
/// (contours after the first in a shape) are preserved through the
/// union, since `i_overlay` and `laserprep_vectorize`'s traced
/// contours share the same exterior/hole winding convention.
///
/// Per `docs/adr/0004-geometry-boolean-ops.md`: this operation cannot
/// produce invalid geometry — `i_overlay`'s self-union always yields a
/// well-formed result (or an empty one for empty input), so there is
/// no failure path to fall back from here.
pub fn merge_adjacent_regions(shapes: &[PolygonShape]) -> Vec<PolygonShape> {
    shapes.to_vec().simplify_shape(FillRule::NonZero)
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    (dx * dx + dy * dy).sqrt()
}

/// Reorders `paths` with a greedy nearest-neighbor heuristic to
/// reduce total travel distance between cuts (CLAUDE.md Section 8:
/// "Path Ordering") — the laser head jumps from the previous path's
/// end point to the next path's start point between cuts, and every
/// path this pipeline produces is closed, so a path's
/// [`VectorPath::start`] is also where the laser ends up after
/// cutting it.
///
/// Starting from the origin (`[0.0, 0.0]`, an arbitrary but consistent
/// reference point — this function has no notion of where the laser
/// actually starts), repeatedly picks whichever remaining path starts
/// closest to the current position. This is the standard greedy
/// approximation to the (NP-hard) shortest-path ordering problem, not
/// an optimal solution — see [`score_path_order`] for how close a
/// given order gets to it. `O(n²)`, which is fine for the path counts
/// a single tone layer produces; a spatial index would only pay for
/// itself at path counts this pipeline doesn't reach in practice.
pub fn order_paths_by_travel_distance(paths: &[VectorPath]) -> Vec<VectorPath> {
    let mut remaining: Vec<&VectorPath> = paths.iter().collect();
    let mut ordered = Vec::with_capacity(paths.len());
    let mut current = [0.0, 0.0];

    while !remaining.is_empty() {
        let (nearest_index, nearest) = remaining
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                distance(current, a.start).total_cmp(&distance(current, b.start))
            })
            .expect("remaining is non-empty inside the loop condition");

        current = nearest.start;
        ordered.push((*nearest).clone());
        remaining.remove(nearest_index);
    }

    ordered
}

/// How much travel distance a given path order costs, relative to the
/// greedy-optimal order [`order_paths_by_travel_distance`] would
/// produce for the same paths (CLAUDE.md Section 8: exposing a "Laser
/// Optimization Score" for path ordering — `docs/roadmap.md` Phase 5).
///
/// Scoped deliberately to inter-path travel only: total cut length
/// (tracing the paths themselves) doesn't change with ordering, so it
/// isn't part of this score. `efficiency` of `1.0` means the given
/// order already achieves the greedy-optimal travel distance (which is
/// always true right after calling [`order_paths_by_travel_distance`]
/// — the two travel distances become equal, since re-running the same
/// deterministic greedy heuristic on an already-optimal order
/// reproduces it); lower means there is travel distance to save by
/// reordering.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaserOptimizationScore {
    /// Total distance, in source-pixel units, jumped between
    /// consecutive paths' start points in the order given (plus the
    /// jump from the origin to the first path).
    pub travel_distance: f64,
    /// The same total, for the greedy-optimal order of the same paths.
    pub optimal_travel_distance: f64,
    /// `optimal_travel_distance / travel_distance`, clamped to
    /// `0.0..=1.0`. `1.0` when there is no travel to save (including
    /// the trivial cases of zero or one path, where there is nothing
    /// to order).
    pub efficiency: f64,
}

/// Scores `paths` in the order given — see [`LaserOptimizationScore`].
pub fn score_path_order(paths: &[VectorPath]) -> LaserOptimizationScore {
    score_from_totals(
        total_travel_distance(paths),
        total_travel_distance(&order_paths_by_travel_distance(paths)),
    )
}

/// Combines several [`LaserOptimizationScore`]s — typically one per
/// tone layer, since path ordering is applied within each tone layer
/// independently — into one score for the whole document. Travel
/// distances sum; `efficiency` is recomputed from the sums rather than
/// averaged across scores, so a layer with more paths correctly
/// outweighs a layer with few when they disagree.
pub fn combine_scores(
    scores: impl IntoIterator<Item = LaserOptimizationScore>,
) -> LaserOptimizationScore {
    let (travel_distance, optimal_travel_distance) =
        scores
            .into_iter()
            .fold((0.0, 0.0), |(travel, optimal), score| {
                (
                    travel + score.travel_distance,
                    optimal + score.optimal_travel_distance,
                )
            });

    score_from_totals(travel_distance, optimal_travel_distance)
}

fn score_from_totals(travel_distance: f64, optimal_travel_distance: f64) -> LaserOptimizationScore {
    let efficiency = if travel_distance <= 0.0 {
        1.0
    } else {
        (optimal_travel_distance / travel_distance).clamp(0.0, 1.0)
    };

    LaserOptimizationScore {
        travel_distance,
        optimal_travel_distance,
        efficiency,
    }
}

fn total_travel_distance(paths: &[VectorPath]) -> f64 {
    let mut total = 0.0;
    let mut current = [0.0, 0.0];
    for path in paths {
        total += distance(current, path.start);
        current = path.start;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, size: f64) -> Vec<[f64; 2]> {
        vec![
            [x0, y0],
            [x0 + size, y0],
            [x0 + size, y0 + size],
            [x0, y0 + size],
        ]
    }

    #[test]
    fn merges_two_overlapping_squares_into_one_shape() {
        let shapes = vec![vec![square(0.0, 0.0, 4.0)], vec![square(2.0, 0.0, 4.0)]];

        let merged = merge_adjacent_regions(&shapes);

        assert_eq!(merged.len(), 1, "overlapping squares should merge");
    }

    #[test]
    fn leaves_separate_regions_unmerged() {
        let shapes = vec![vec![square(0.0, 0.0, 2.0)], vec![square(10.0, 10.0, 2.0)]];

        let merged = merge_adjacent_regions(&shapes);

        assert_eq!(merged.len(), 2, "far-apart squares should stay separate");
    }

    #[test]
    fn preserves_a_hole_through_the_merge() {
        let ring = vec![
            square(0.0, 0.0, 10.0),
            vec![[3.0, 3.0], [3.0, 7.0], [7.0, 7.0], [7.0, 3.0]],
        ];
        let shapes = vec![ring];

        let merged = merge_adjacent_regions(&shapes);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].len(),
            2,
            "the hole must survive as its own contour"
        );
    }

    #[test]
    fn an_empty_input_produces_no_shapes() {
        assert!(merge_adjacent_regions(&[]).is_empty());
    }

    fn path_at(start: [f64; 2]) -> VectorPath {
        VectorPath {
            svg_element: String::new(),
            start,
        }
    }

    #[test]
    fn orders_paths_by_nearest_neighbor_from_the_origin() {
        // Starting at the origin, the nearest-first order should visit
        // these left-to-right, even though the input interleaves them.
        let paths = vec![
            path_at([10.0, 0.0]),
            path_at([1.0, 0.0]),
            path_at([5.0, 0.0]),
        ];

        let ordered = order_paths_by_travel_distance(&paths);

        assert_eq!(
            ordered.iter().map(|p| p.start).collect::<Vec<_>>(),
            vec![[1.0, 0.0], [5.0, 0.0], [10.0, 0.0]]
        );
    }

    #[test]
    fn ordering_never_drops_or_duplicates_a_path() {
        let paths = vec![
            path_at([3.0, 3.0]),
            path_at([-5.0, 2.0]),
            path_at([0.0, 0.0]),
            path_at([100.0, -20.0]),
        ];

        let ordered = order_paths_by_travel_distance(&paths);

        assert_eq!(ordered.len(), paths.len());
        for original in &paths {
            assert_eq!(
                ordered.iter().filter(|p| p.start == original.start).count(),
                1
            );
        }
    }

    #[test]
    fn ordering_an_empty_or_single_path_list_is_a_no_op() {
        assert!(order_paths_by_travel_distance(&[]).is_empty());

        let one = vec![path_at([7.0, 7.0])];
        assert_eq!(order_paths_by_travel_distance(&one), one);
    }

    #[test]
    fn scores_an_already_optimal_order_at_full_efficiency() {
        let paths = vec![
            path_at([1.0, 0.0]),
            path_at([5.0, 0.0]),
            path_at([10.0, 0.0]),
        ];

        let score = score_path_order(&paths);

        assert_eq!(score.travel_distance, score.optimal_travel_distance);
        assert_eq!(score.efficiency, 1.0);
    }

    #[test]
    fn scores_a_backwards_order_below_full_efficiency() {
        // Visiting far-then-near-then-far costs more travel than the
        // greedy near-to-far order the score compares against.
        let paths = vec![
            path_at([10.0, 0.0]),
            path_at([1.0, 0.0]),
            path_at([5.0, 0.0]),
        ];

        let score = score_path_order(&paths);

        assert!(score.travel_distance > score.optimal_travel_distance);
        assert!(score.efficiency < 1.0);
    }

    #[test]
    fn reordering_a_path_list_raises_its_score_to_full_efficiency() {
        let paths = vec![
            path_at([10.0, 0.0]),
            path_at([1.0, 0.0]),
            path_at([5.0, 0.0]),
        ];
        let reordered = order_paths_by_travel_distance(&paths);

        let score = score_path_order(&reordered);

        assert_eq!(score.efficiency, 1.0);
    }

    #[test]
    fn scores_no_paths_at_full_efficiency() {
        let score = score_path_order(&[]);
        assert_eq!(score.travel_distance, 0.0);
        assert_eq!(score.efficiency, 1.0);
    }

    #[test]
    fn combining_scores_sums_their_travel_distances() {
        let a = LaserOptimizationScore {
            travel_distance: 10.0,
            optimal_travel_distance: 10.0,
            efficiency: 1.0,
        };
        let b = LaserOptimizationScore {
            travel_distance: 20.0,
            optimal_travel_distance: 10.0,
            efficiency: 0.5,
        };

        let combined = combine_scores([a, b]);

        assert_eq!(combined.travel_distance, 30.0);
        assert_eq!(combined.optimal_travel_distance, 20.0);
        assert_eq!(combined.efficiency, 20.0 / 30.0);
    }

    #[test]
    fn combining_no_scores_is_full_efficiency_with_no_travel() {
        let combined = combine_scores(Vec::<LaserOptimizationScore>::new());
        assert_eq!(combined.travel_distance, 0.0);
        assert_eq!(combined.efficiency, 1.0);
    }
}
