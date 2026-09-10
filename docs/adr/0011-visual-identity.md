# ADR 0011: Visual identity implementation, self-hosted font, phased rollout

## Status

Accepted (Phase 1 of a multi-phase rollout — see "Consequences")

## Context

Through Phase 7, LaserPrep's frontend had zero CSS — no stylesheet
file, no CSS import anywhere, no favicon. Every element rendered with
default browser/OS form-control styling. This was appropriate while
the priority was functional completeness (Phases 0-7), but the app
had no visual identity at all.

The user commissioned a design via Claude Design (claude.ai/design)
and handed off the resulting bundle: a color palette (Laser Red/Laser
Orange on Graphite/Off-White), an IBM Plex Mono type system, an app
icon (dot → laser beam → vector triangles, representing raster→SVG
conversion), a wordmark and horizontal logo, six preset pictograms, UI
component specs (buttons/inputs/cards), and mockups for a splash
screen, GitHub social preview, and Windows installer. The handoff's
own README instructed recreating the designs "pixel-perfectly... in
whatever technology makes sense for the target codebase" rather than
copying the HTML/CSS prototype's structure, and to ask before
implementing if scope was ambiguous.

Given the scope (icon assets, a full CSS design system, React
component styling, installer/README assets, a splash screen) touches
several independent parts of the codebase with very different risk
profiles, the user was asked how to scope the rollout and chose:
implement everything, but in small phases through the project's normal
auto-ci workflow (branch → implement → test → PR → CI → merge),
proceeding autonomously through each phase.

## Decisions

- **Self-hosted font, not the handoff's Google Fonts `@import`.**
  `@fontsource/ibm-plex-mono` (weights 400/500/600/700, matching the
  design spec) ships the font files in the app bundle instead of
  fetching them from `fonts.googleapis.com` at runtime. CLAUDE.md
  Section 21 defaults to no network dependency, and a maker's workshop
  — this app's actual use case — is exactly the kind of place that
  doesn't reliably have internet; the app must render correctly
  offline regardless.
- **CSS custom properties + a `data-theme` attribute for
  light/dark/system**, not `prefers-color-scheme` alone. LaserPrep
  already modeled `settings.ui.theme: "light" | "dark" | "system"` in
  `crates/settings` (CLAUDE.md Section 23) but nothing in the frontend
  ever read it — the setting round-tripped through the settings file
  with zero effect. `src/index.css` defines the light palette on bare
  `:root`, redefines it under `@media (prefers-color-scheme: dark)`
  guarded by `:root:not([data-theme="light"])` (so "system" tracks the
  OS), and redefines it again under `:root[data-theme="dark"]` (so an
  explicit choice wins in both directions). A new effect in `App.tsx`
  sets/clears that attribute from `settings.ui.theme`, and a new
  "Theme" selector (mirroring the existing language switcher) lets a
  user actually change it — completing a feature that was already
  half-built rather than introducing a new one.
- **Global tag-selector CSS first, component classes later.** Because
  the app had zero CSS, styling `body`, `h1`-`h3`, `button`, `input`,
  `select`, `label` etc. in `src/index.css` changes the entire app's
  appearance without touching a single line of `App.tsx`'s JSX
  structure — lowest possible risk for the highest visual return.
  Class-based component styling (`.card`, `.btn-secondary`, per-preset
  icons in the batch/preset UI) is deferred to a later phase precisely
  because it requires touching many individual call sites in a
  1200+-line component file with 42 existing tests to keep green.
- **App icon regenerated via `npx tauri icon` from the handoff SVG**,
  then pruned to only the targets this project actually ships
  (`icon.ico`, `icon.png`, `32x32.png`, `64x64.png`, `128x128.png`,
  `128x128@2x.png`). `tauri icon` defaults to generating macOS/iOS/
  Android/Windows Store (MSIX) assets too; none apply here — see
  README's "Supported platforms" and `tauri.conf.json`'s
  `bundle.targets: ["msi", "nsis"]`. Kept out of the repo rather than
  committed-then-ignored, matching auto-ci Standard 9 ("generated
  artifacts must not normally be committed") in spirit — these are
  build *inputs* checked in because `tauri build` reads them directly,
  not build outputs, but the platform sets Tauri generates and this
  project doesn't ship are neither.
- **Wordmark/logo SVGs needed a fix, not a verbatim copy.** The
  handoff's `<text>` elements for "LaserPrep" carried no
  `font-family`/`font-weight`/`font-size` of their own — they relied
  on the design *document's* ambient page CSS, which doesn't exist
  once the SVG is a standalone file. Added explicit font attributes
  (IBM Plex Mono, 700, matching the design spec's declared sizes) so
  the assets are self-contained and render correctly wherever they're
  used, per the handoff README's own instruction to match visual
  output rather than copy prototype structure that doesn't fit.
- **Stripped the embedded C2PA content-credentials metadata** from
  each SVG (a large base64 provenance blob Claude Design embeds
  automatically) before committing — irrelevant once the asset lives
  in the repo, and bloats every diff touching these files.

## Consequences

- Phase 1 (this change): design tokens, self-hosted font, global base
  styles, app icon, header logo, and the theme setting's first real
  UI. Ships with the existing 42 frontend tests unmodified and green —
  confirms the styling changes didn't alter any accessible name, role,
  or behavior the tests depend on.
- Deferred to later phases, tracked as follow-up work rather than
  silently dropped: per-component class styling (cards, secondary
  buttons, the six preset pictograms in the preset picker), the splash
  screen, the GitHub social preview image, and Windows installer
  branding (NSIS/MSI header/sidebar bitmaps).
- The app icon's thin, unfilled stroke style (faithful to the handoff)
  is hard to read at 32×32 and smaller — flagged to the user as a
  legibility concern with the design itself, not silently "fixed" by
  thickening strokes or adding fill, since that's a brand/aesthetic
  call outside an implementation decision.
- IBM Plex Mono has no CJK glyphs, so `zh-CN` UI text falls back to
  the OS's default CJK font automatically (standard per-character font
  fallback) — expected, not a bug, and not something a monospace
  Latin/Cyrillic/Vietnamese font family could cover regardless of
  which one was chosen.
