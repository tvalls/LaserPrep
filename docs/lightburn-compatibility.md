# LightBurn compatibility

> Stub — expanded in Phase 2 alongside the SVG validator
> (`crates/svggen`). See CLAUDE.md Sections 8-10 for the source
> requirements.

Planned sections:

- SVG features avoided for compatibility: filters, complex masks, blend
  modes, proprietary/vendor extensions.
- Tone-group convention (`<g id="tone-N">`) and how it maps to LightBurn
  layers.
- Legend group structure and how to hide/show it in LightBurn.
- Metadata fields embedded in exported SVGs (software, version, date,
  preset, tone count, parameters, dimensions, source DPI, algorithm
  version) and where LightBurn surfaces them, if at all.
- The "Validate LightBurn compatibility" check performed by
  `crates/svggen` and how to read its output.
