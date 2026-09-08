# ADR 0005: Locale infrastructure ships four locales; UI exposes two at launch

## Status

Accepted

## Context

Two requirements interact here:

- CLAUDE.md Section 23 (product scope, owner-level decision): "Português
  do Brasil e Inglês desde o início, como idiomas de primeira classe" —
  pt-BR and English are the first-class, officially supported languages
  from day one.
- The auto-ci skill's Standard 5 (global engineering workflow, applies to
  every generated user-facing program): ships `en-US`, `pt-BR`, `es`, and
  `zh-CN` locale resources, with all four files containing the same key
  set, validated in CI.

These are not actually in conflict once separated into two different
concerns: which locale *resource files exist and are structurally valid*
(an engineering/workflow standard) versus which locales are *marketed and
QA'd as officially supported* (a product decision).

## Decision

- `locales/en-US.json`, `locales/pt-BR.json`, `locales/es.json`, and
  `locales/zh-CN.json` all exist from the start, are all real (translated,
  not placeholder/fake) content, and are all validated in CI for key
  parity (Standard 5, rule 10).
- `en-US` is the canonical source locale; `pt-BR` is fully translated and
  treated as equally first-class (per CLAUDE.md), also maintained
  translation-complete at every change.
- `es` and `zh-CN` are kept translation-complete as an engineering
  standard, but are **not surfaced in the language switcher UI** until a
  later phase where they can be reviewed for quality/completeness beyond
  "translated, not proofread by a native/target-market reviewer." The
  runtime fallback chain (exact locale → base language → `en-US`) still
  works correctly for them if a user's OS locale matches.
- This keeps the "no fake features" rule (CLAUDE.md Section 13) intact:
  nothing in the switcher claims support that hasn't been reviewed, but
  nothing in the codebase is stubbed/fake either.

## Consequences

- Every user-facing string change touches four JSON files, not two.
- Promoting `es`/`zh-CN` to "officially supported" later is a UI-only
  change (add them to the switcher) plus a translation-quality pass — not
  a re-architecture.
