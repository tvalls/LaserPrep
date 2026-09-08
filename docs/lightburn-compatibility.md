# LightBurn compatibility

## SVG features avoided for compatibility

LaserPrep's generated SVGs never contain filters, complex masks/clips,
blend modes, embedded scripts, embedded rasters, or other proprietary/
renderer-specific constructs — LightBurn's SVG importer does not
reliably support them. `laserprep_svggen::validate` checks a generated
document for the following markers and reports any it finds:

- `<filter`
- `<mask`
- `<clipPath`
- `<foreignObject`
- `<script`
- `<image`
- `mix-blend-mode`

A document is LightBurn-compatible (`ValidationReport::is_lightburn_compatible`)
when none of these are present.

## Tone-group convention

Each tone level is its own `<g id="tone-N">` group (`N` from `0` to
`tone_count - 1`), containing that tone's traced `<path>` elements.
LightBurn imports each top-level `<g>` as a separate layer, so a
tone-N group becomes a layer an operator can assign a distinct power/
speed to.

## Legend group structure

Optional (`SvgOptions::include_legend`, off by default). When enabled,
`laserprep_svggen::vectorized_tones_to_svg` appends a `<g id="legend">`
group after the tone-N artwork groups: one filled swatch plus its tone
index per row, positioned to the right of the artwork (outside the
main art, per CLAUDE.md Section 10). Being its own top-level `<g>`, it
is independently hideable/editable as a LightBurn layer, same as any
`tone-N` group. The document's width/viewBox grow to fit it; validator
output (`ValidationReport`) is unaffected since it only counts
`<path>` elements, and the legend uses `<rect>`/`<text>`.

## Embedded metadata

Every generated document has a `<metadata>` element recording the
LaserPrep version, tone count, source DPI, and the algorithm
identifier (currently `quantize-linear+vtracer`). LightBurn does not
surface arbitrary SVG metadata in its UI; this is for diagnostics
(`docs/development.md`) and future project-file round-tripping
(`crates/project`, Phase 4), not for the operator.

## The SVG validator

`laserprep_svggen::validate(svg, width, height, tone_count)` returns a
`ValidationReport` with:

| Field | Meaning |
|---|---|
| `total_paths` | Number of `<path>` elements found |
| `closed_paths` / `open_paths` | Paths ending in `Z`/`z` vs. not. CLAUDE.md Section 8's default goal is zero open paths in filled regions — `laserprep_vectorize::VtracerVectorizer` always closes its traced paths, so `open_paths` should always be `0` for LaserPrep's own pipeline output |
| `degenerate_paths` | Paths with at most one node (draw nothing) |
| `total_nodes` | Total `M`/`L`/`C` commands across all paths |
| `invalid_coordinate_paths` | Paths containing a non-finite (NaN/infinite) coordinate |
| `lightburn_incompatibilities` | Which of the markers above were found, if any |

Convenience checks: `has_no_open_paths()`, `has_valid_coordinates()`,
`is_lightburn_compatible()`.

**Not implemented yet:**

- **Self-intersection detection** — needs real segment/curve geometry;
  `VectorPath` is intentionally a plain `<path>` element string (see
  `docs/adr/0003-vectorization-library.md`), not a geometric model, so
  this is deferred until there's a concrete need for it.
- **"Duplicates removed" / "tiny areas removed" counts** — these
  describe what `crates/optimize` removes, and that crate has no logic
  yet (a later Phase 2 item). Not faked as always-zero here.
