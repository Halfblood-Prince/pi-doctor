# Changelog

This project follows semantic versioning before `1.0.0`, with the CLI and JSON
schema contracts versioned explicitly.

## Unreleased

### Added

- Security policy with private vulnerability-reporting guidance.
- Contribution guide, maintainer policy, release-process documentation, known
  limitations, and issue templates.
- Privacy-preserving structured log format for operational debugging via
  `PI_DOCTOR_LOG_FORMAT=json`.

### Fixed

- CI formatting regressions reported by `cargo fmt --all --check`.
- Rust ownership error in supported OS detection.
- Supply-chain workflow pin for `cargo-audit` to a version that supports newer
  RustSec advisory metadata.

## 0.1.0 - 2026-06-23

### Added

- Initial read-only Raspberry Pi diagnostic CLI.
- JSON report schema, finding registry, focused doctor commands, support
  bundles, Debian packaging, release archive tooling, and MkDocs site.
