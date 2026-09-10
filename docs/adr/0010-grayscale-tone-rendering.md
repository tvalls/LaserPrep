# ADR 0010: Tone groups render in representative grayscale, not flat black

## Status

Accepted

## Context

Every tone group (`<g id="tone-N">`) and legend swatch rendered as flat
black (`fill="#000000"`), regardless of how dark or light that tone
actually was in the source photo. A user testing the app on a real
photo reported the exported SVG as unusable for engraving on
inspection — investigating that report surfaced a real vectorization
noise problem (documented separately), but also this: opening the raw
SVG showed an undifferentiated black silhouette, with no visual cue
for which tone was meant to be the darkest burn and which the
lightest. Regulating power/speed per layer in LightBurn meant guessing.

CLAUDE.md Sections 7 and 12 are explicit that LaserPrep must never
presume laser power/speed — that depends on material and machine, and
belongs entirely to LightBurn. Any fix had to stay strictly on the
"rendering" side of that boundary: helping a user *see* relative
darkness, never telling them what power/speed number to use.

Options considered:

- **Show suggested power/speed percentages per tone.** Rejected
  outright — this is exactly what Sections 7/12 forbid. Even framed as
  "just a starting point," it would be laser-power guidance coming
  from software that has no idea what material or machine the user
  has.
- **Numeric luminance/percentage labels in the legend only**, artwork
  still flat black. Would help a user compare tones in the legend, but
  the artwork itself — what they'd actually screenshot, print, or
  paste into LightBurn as a visual reference — would still look like
  an undifferentiated black shape.
- **Render each tone group (and its legend swatch) in that tone's own
  representative grayscale**, computed only from the source image's
  own pixel data. Chosen: purely a `fill` value, decided per tone from
  data the pipeline already has (the quantized `ToneMap` plus the
  original luminance samples) — no new user input, no assumption about
  hardware, nothing that could be mistaken for a power/speed
  recommendation.

## Decision

Compute each tone's mean source luminance
(`laserprep_quantize::tone_mean_luminance`, new) from the same
`ToneMap` and luminance buffer `quantize_linear` already produces —
the actual average of the pixels assigned to that tone, not the linear
band's midpoint, so the result reflects the image's real histogram
rather than an evenly-spaced synthetic ramp. `crates/pipeline` computes
this once per conversion and passes it into
`laserprep_svggen::vectorized_tones_to_svg`, which now takes a
`tone_grays: &[u8]` parameter and sets `fill="#rrggbb"` on each
`<g id="tone-N">` and legend `<rect>` swatch instead of a hardcoded
`#000000`.

To make this possible, `laserprep_vectorize::VectorPath` no longer
bakes a `fill` into each traced `<path>` element — a single traced
region only knows its own tone's binary mask, not how that tone's
color should relate to the *other* tones in the document, so it has no
basis to pick one. Color moved to where the cross-tone information
actually lives: `crates/svggen`, applied once per tone group and
inherited by that group's child `<path>` elements (standard SVG
presentation-attribute inheritance — individual paths carry no `fill`
of their own to override it).

The desktop app's existing "Colorize by Layer" preview toggle
(`buildPreviewSvg` in `src/App.tsx`) needed no change: it injects a
`<style>` rule targeting `#tone-N path` when active, which has higher
CSS specificity than the `<g>`'s presentation attribute and overrides
it as before; with the toggle off, the pristine grayscale fill shows
through unchanged. Export/save already always used the pristine SVG.

## Consequences

- Opening a generated SVG directly now shows a recognizable, posterized
  grayscale rendition of the source photo — darkest tone genuinely
  darkest, lightest genuinely lightest, matching the actual image
  rather than an arbitrary index order.
- `vectorized_tones_to_svg` and `legend_fragment` both gained a
  required `tone_grays: &[u8]` parameter; a missing or out-of-range
  index falls back to black (same as a missing `paths_by_tone` index
  already did) rather than panicking.
- `VectorPath::svg_element` no longer contains `fill` for either
  vectorization path (`VtracerVectorizer::trace` or
  `polygon_shape_to_vector_path`) — a caller relying on that
  attribute's presence (none did, in this codebase) would need to
  apply its own.
- No new dependency, no new user-facing parameter, no change to path
  geometry — the SVG validator (`crates/svggen::validate`) is
  unaffected since it only inspects `<path>` elements, not `fill`.
