# ADR 0001: Project license — GPL-3.0-or-later

## Status

Accepted

## Context

LaserPrep is an open-source, community-oriented desktop tool for laser
engraving hobbyists and makers. Before any release-capable CI/CD pipeline
can be wired up, the project needs a definitive license, since it affects
dependency compatibility, redistribution terms, and how derivative tools
may be built on top of LaserPrep.

Options considered: MIT, Apache-2.0, GPL-3.0-or-later.

## Decision

LaserPrep is licensed under **GPL-3.0-or-later**.

This was a product/business decision made by the project owner (not a
technical implementation detail), given LaserPrep's target audience is the
maker/hobbyist community, where copyleft licensing is common and where
keeping downstream forks and derivatives open-source is valued over
maximizing adoption by proprietary/closed integrators.

## Consequences

- Any dependency added to the project must be compatible with GPL-3.0
  (permissive licenses such as MIT/Apache-2.0/BSD are compatible; other
  copyleft licenses need case-by-case review; proprietary/closed
  dependencies are disallowed).
- Forks and redistributions of LaserPrep must remain open-source under a
  GPL-3.0-compatible license.
- `LICENSE` at the repository root contains the verbatim license text from
  the Free Software Foundation.
