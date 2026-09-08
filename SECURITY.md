# Security Policy

## Supported Versions

LaserPrep is pre-1.0 and under active development. Only the latest
released version receives security fixes until a stable 1.0 release
policy is published here.

## Reporting a Vulnerability

Please **do not open a public GitHub issue** for security vulnerabilities.

Instead, use GitHub's private vulnerability reporting for this repository
(Security tab → "Report a vulnerability"). If that is not available,
open a minimal public issue asking a maintainer to contact you privately,
without describing the vulnerability itself.

Please include:

- a description of the vulnerability and its potential impact
- steps to reproduce (a minimal malicious/malformed input file if
  relevant — see "Malformed input files" below)
- affected version/commit

We aim to acknowledge reports within 5 business days.

## Scope

LaserPrep is a local desktop application. Relevant security concerns
include:

- parsing of imported raster images and SVG files (malformed/malicious
  input causing crashes, excessive memory/CPU use, or path traversal)
- the auto-update mechanism (verifying it cannot be tricked into
  installing unverified/tampered payloads)
- settings/project file (`.lvp`) parsing

LaserPrep does not run a network service and does not transmit user
images anywhere by default (see the Privacy section of the README), which
significantly limits remote attack surface.

## Malformed Input Files

If you find an image, SVG, or `.lvp` project file that crashes the
application, causes a hang, or consumes unreasonable memory, please
report it as above. Do not attach personal/sensitive images to public
reports.
