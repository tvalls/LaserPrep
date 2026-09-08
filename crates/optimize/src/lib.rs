//! Laser path optimization: closing, noise/island removal, merging,
//! simplification, ordering. See CLAUDE.md Section 8 and
//! `docs/adr/0004-geometry-boolean-ops.md`.
//!
//! Implemented so far: [`merge_adjacent_regions`] (boolean union via
//! `i_overlay`). Path closing, noise/island removal, and curve
//! simplification are already handled upstream by
//! `laserprep_vectorize::VtracerVectorizer` (always-closed traced
//! paths; the "Minimum Area" parameter). Path ordering is Phase 5
//! (`docs/roadmap.md`), not implemented here yet.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::simplify::SimplifyShape;
use laserprep_vectorize::PolygonShape;

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
}
