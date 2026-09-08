# Image processing

See `docs/architecture.md` for the conceptual pipeline and `docs/adr/`
for the standing technical decisions (no ML, `vtracer` usage, geometry
libraries).

## Heuristic feature extraction (`crates/analysis`)

`laserprep_analysis::analyze(&RgbImage) -> ImageFeatures` is a pure
function over pixel data — no I/O, no randomness, no external model
(CLAUDE.md Section 2). It computes:

| Field | Meaning |
|---|---|
| `aspect_ratio` | `width / height` |
| `mean_luminance` | Mean ITU-R BT.601 luma, `0.0`-`255.0` |
| `contrast` | Population standard deviation of luminance — a low-cost global contrast estimate |
| `dynamic_range` | `max luminance - min luminance` actually present |
| `mean_saturation` | Mean HSV saturation, `0.0` (grayscale) to `1.0` (fully saturated) |
| `uniform_background_ratio` | Fraction of border pixels within `24.0` (Euclidean RGB distance) of the border's mean color — high for a plain studio/product background, low for a busy or textured one |
| `edge_density` | Fraction of interior pixels flagged as an edge by a Sobel operator on luminance (gradient magnitude threshold `80.0`) |

Not implemented yet: color clustering (k-means) and a symmetry score.
CLAUDE.md Section 6 mentions both, but neither was needed for the
categories `classify` currently distinguishes (below) — they're
tracked as a follow-up, not faked.

## Rule-based classification

`laserprep_analysis::classify(&ImageFeatures) -> Classification` picks
the highest-scoring `ContentCategory` from a small set of hand-written
threshold rules, falling back to `GenericPhoto` when none fire.
`confidence` is the winning rule's own score (`0.0..=1.0`), not a
calibrated statistical probability.

| Category | Rule (all thresholds are simple hand-picked constants, not fitted) |
|---|---|
| `UniformBackground` | `uniform_background_ratio > 0.9` **and** `edge_density < 0.02` — the whole image is essentially flat, not just its border |
| `Logo` | `uniform_background_ratio > 0.75` **and** `0.02 <= edge_density < 0.3` — a mark/subject isolated on a plain background, scored by `uniform_background_ratio * min(contrast / 128, 1)` |
| `Landscape` | `aspect_ratio > 1.3` **and** `uniform_background_ratio < 0.6` — wide framing with content filling the frame edge to edge |
| `ComplexBackground` | `uniform_background_ratio < 0.5` — busy/textured content everywhere, regardless of aspect ratio |
| `GenericPhoto` | Fallback baseline score `0.25`; wins whenever no rule above scores higher |

### Scope limitation (read before extending this list)

CLAUDE.md Section 6 lists many more categories: portrait (single/
multiple people), full-body person, pet, animal, product/object,
drawing, illustration, icon, text, architecture, vehicle, and more.
`classify` does not attempt any of them. This is deliberate, not an
oversight: telling a portrait from an animal from a vehicle needs
locating and recognizing a *subject* in the frame, and every classical
technique capable of that (Haar cascades, any trained classifier/CNN)
is exactly what CLAUDE.md Section 2 forbids. A global image statistic
— everything in the table above — has no way to do this honestly.

If a future heuristic genuinely separates one of these categories from
the others using only classical, non-trained signals (CLAUDE.md
Section 6's suggested "largest low-saturation central blob" test for a
portrait subject is one candidate, not yet implemented or validated),
add it as a new rule here with the same treatment: a documented
threshold, a synthetic-fixture test proving it actually fires, and an
honest account of what it still can't tell apart. Don't add a category
"guess" that isn't backed by a real, testable signal.

## Presets (`crates/presets`)

`laserprep_presets::suggest_preset(ContentCategory) -> Preset` maps
only the categories above to a starting parameter bundle (tone count,
minimum area, legend, merge-adjacent — never laser power/speed, which
belongs to LightBurn/the machine). `Preset::PORTRAIT`, `Preset::ANIMAL`,
and `Preset::DRAWING` exist and remain manually selectable, but are
never auto-suggested, for the same reason `classify` doesn't detect
those categories.

## Tone quantization (`crates/quantize`)

Linear quantization only so far (`quantize_linear`, Phase 1).
Perceptual, adaptive, histogram-based, and per-region strategies plus
manual threshold editing (CLAUDE.md Section 7) are not implemented
yet.

## Per-category pipelines

Not implemented. Today every image goes through the same pipeline
(quantize → vectorize → optimize → svggen) regardless of its detected
category; only the *parameters* fed into that one pipeline vary, via
presets. Category-specific *pipeline* logic (e.g. a genuinely different
processing strategy for logos vs. photographs, per CLAUDE.md Section 6)
is a later Phase 3 iteration.
