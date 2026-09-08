//! SVG document generation (per-tone `<g id="tone-N">` groups, optional
//! legend, embedded metadata) and the SVG validator (path counts,
//! closed/open/degenerate paths, self-intersections, node counts,
//! LightBurn-compatibility checks). See CLAUDE.md Sections 9-10 and
//! `docs/lightburn-compatibility.md`. Implemented in Phase 1 (basic SVG)
//! and Phase 2 (legend + validator).
