# Installer branding assets

Source: the "LaserPrep visual identity" Claude Design handoff (see
`docs/adr/0011-visual-identity.md`), composed at each target's exact
required pixel size rather than scaled from a single master asset —
NSIS and WiX (MSI) both require plain 24-bit BMP at fixed dimensions,
so cropping/scaling after the fact only loses quality for no benefit.

| File | Used by | Dimensions | Config |
|---|---|---|---|
| `nsis-header.bmp` | NSIS installer/uninstaller header | 150×57 | `bundle.windows.nsis.headerImage` |
| `nsis-sidebar.bmp` | NSIS Welcome/Finish pages | 164×314 | `bundle.windows.nsis.sidebarImage` |
| `msi-banner.bmp` | MSI installer banner (all but first page) | 493×58 | `bundle.windows.wix.bannerPath` |
| `msi-dialog.bmp` | MSI Welcome/Completion dialogs | 493×312 | `bundle.windows.wix.dialogImagePath` |

## Regenerating

`svg/*.svg` are the editable sources (one per target, at its exact
final pixel size — a nested `<svg viewBox="0 0 256 256">` holds the
app icon so it scales cleanly regardless of composition size).

```powershell
# 1. Rasterize each SVG to PNG at its native size (pure-Rust, precise,
#    no browser/ImageMagick dependency):
cargo install resvg --locked
resvg svg\nsis-header.svg png\nsis-header.png
# ...repeat for the other three.

# 2. Convert PNG -> 24-bit BMP (NSIS/MSI don't accept PNG directly).
#    image::DynamicImage::to_rgb8() before saving strips any alpha
#    channel installers don't expect. See the project's own
#    `image` dependency (already used by crates/imaging) — any small
#    throwaway Rust binary calling `image::open(..).to_rgb8().save(..)`
#    does this; there is no dedicated crate in this repo for it since
#    it only runs when these assets change.
```
