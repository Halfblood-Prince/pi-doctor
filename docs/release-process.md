# Release Process

## Versioning Rules

`pi-doctor` uses semantic versioning for package releases:

- Patch releases fix bugs, documentation, packaging metadata, and security
  issues without changing the machine contract.
- Minor releases may add commands, fields, findings, probes, and supported
  hardware.
- Major releases are reserved for intentionally breaking CLI or JSON behavior.

The project must not declare `1.0.0` until:

- the CLI command surface is frozen for normal users
- JSON schema v1 field meanings are stable in practice
- finding IDs, impacts, and remediation categories have a published registry
- release candidates have passed staged hardware validation

Machine-readable removals or semantic changes require a schema bump and release
notes, even before package `1.0.0`.

## Release Candidates

Every public release should go through release candidates:

1. Set `workspace.package.version` in `Cargo.toml` to `X.Y.Z-rc.1`.
2. Tag `vX.Y.Z-rc.1`.
3. Build release archives, checksums, and SBOMs.
4. Run CI, Supply Chain, Docs, and the release workflow.
5. Run native hardware validation on supported Raspberry Pi OS images.
6. Attest and publish only after native hardware validation passes.
7. Publish notes listing known limitations and validation gaps.
8. Repeat with `rc.2`, `rc.3`, and so on until blockers are closed.

Release candidates are not promoted to Debian, Homebrew, or APT channels.

## Staged Validation

Before a final release:

- validate `pi-doctor --json check` against the schema
- confirm the Git tag version matches `workspace.package.version` in `Cargo.toml`
- run subsystem fixtures and parser corpus tests
- test exact release archives, not only local build outputs
- pass native Raspberry Pi release validation for Pi 3, Pi 4, Pi 5, Zero 2 W,
  Compute Module, camera, no-camera, and thermal fixture runners
- verify archive checksums, SBOMs, and provenance attestations
- test upgrade, downgrade, rollback, and uninstall paths
- confirm known limitations are current

## Final Release

For final release tags:

1. Update `CHANGELOG.md`.
2. Confirm version numbers in Cargo, Debian metadata, docs, and release scripts.
3. Build signed artifacts and publish checksums, SBOM, and attestations.
4. Verify GitHub release assets from a clean machine.
5. Update package channels only after artifact verification passes.
