# Release Engineering

Release sequencing, semantic versioning, release candidates, and staged
validation are defined in [Release Process](release-process.md).

## Toolchain

The supported Rust toolchain is locked in `rust-toolchain.toml` at Rust 1.88.0.
CI installs that toolchain explicitly and uses the same version for formatting,
linting, tests, release builds, and supply-chain checks.

## Release Archives

`scripts/package-release.sh` is designed for reproducible archives:

- `SOURCE_DATE_EPOCH` controls file mtimes and tar metadata.
- archive entries are sorted by path.
- archive ownership is normalized to uid/gid 0.
- gzip is written with `-n` so timestamp and original filename headers are not embedded.
- release CI compares two archives built from the same inputs.

Release targets include GNU libc and musl variants:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `armv7-unknown-linux-gnueabihf`
- `armv7-unknown-linux-musleabihf`

GNU libc archives are intended for normal Raspberry Pi OS installs. Musl
archives provide the static-build path where a lower host libc baseline is
needed or easier to validate.

## Artifact Gates

Do not publish package channels until the matching GitHub release includes:

- target-specific archives
- target-specific SHA-256 files
- SBOM files
- provenance attestations
- a release note stating whether hardware validation was run for the supported
  Raspberry Pi OS matrix

The manual hardware validation workflow can test exact published archives by
passing `release_version` in `workflow_dispatch`. It requires matching
self-hosted Raspberry Pi runners and is not part of the hosted tag release
workflow.

The tag release workflow gates publication on hosted release verification:
formatting, Clippy, workspace tests, cargo audit, cargo deny, Debian package
build, lintian, autopkgtest, cross-target release archives, SBOM generation, and
provenance attestation.

## Package Channels

Debian packaging in `debian/` is maintained for mentors review. Publish to
mentors or an APT repository only after the artifact gates are complete.

Homebrew packaging lives in `packaging/homebrew/pi-doctor.rb`. Fill in the
release archive SHA-256 only after the signed release artifact exists.

Optional Snap or Flatpak packaging should be added only if confinement does not
block the read-only hardware diagnostics documented in the CLI contract.

## Installer Lifecycle

CI tests the release installer with real archives for:

- upgrade install
- downgrade install
- rollback to the previous binary
- uninstall

The installer keeps the previous binary at `.pi-doctor.previous` in the target
binary directory and restores it with `--rollback`.

`--skip-attestation` is reserved for explicitly supplied local archives used in
offline or CI smoke tests. Remote GitHub release downloads still require
attestation verification.
