# ADR 0006: Windows code signing (Authenticode) deferred; updater signing is independent

## Status

Accepted

## Context

Two distinct signing mechanisms are relevant to a Tauri desktop app on
Windows:

1. **Authenticode code signing** of the MSI/NSIS installer and the
   application executable — requires a paid certificate (OV or EV) from a
   CA, avoids/reduces Windows SmartScreen warnings, and is what most users
   mean by "signing the app."
2. **Update package signing** (minisign keypair via
   `cargo tauri signer generate`) — used by `tauri-plugin-updater` to
   verify that downloaded update payloads genuinely come from this
   project's release pipeline. This is free and unrelated to Authenticode.

The project owner confirmed (2026-09-08) there is no Authenticode
certificate available yet, and does not want to block Phase 0–5 work on
acquiring one.

## Decision

- Ship without Authenticode signing for now. Installers built by CI will
  trigger Windows SmartScreen's "unknown publisher" warning on first run.
  This is documented in the README's installation instructions so users
  aren't surprised.
- Implement update-package signing (minisign) from the start (Phase 6,
  when the updater is wired up) regardless of Authenticode status — it is
  free, and Standard 6 of the auto-ci skill requires update integrity
  verification independent of platform code signing.
- Acquiring an Authenticode certificate is a prerequisite to evaluate
  before a larger public launch, tracked as a Phase 6 action item, not a
  blocking decision now.

## Consequences

- README must clearly explain the SmartScreen warning and how to proceed
  past it, until this is revisited.
- No architectural change is needed if/when a certificate is purchased
  later — it plugs into the existing `tauri-action`/release workflow as
  an additional signing step.
