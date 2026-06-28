# Maintainer Policy

## Responsibilities

Maintainers keep the project releaseable, private by default, and honest about
hardware coverage. That includes:

- protecting the read-only probe contract
- reviewing privacy impact for logs, support bundles, fixtures, and docs
- keeping CI, schema validation, and supply-chain checks required for releases
- documenting known limitations before release
- avoiding `1.0.0` until the public CLI and JSON contracts are frozen

## Review Rules

Changes should receive extra scrutiny when they touch:

- command execution
- support bundle collection
- redaction
- JSON schema fields
- finding IDs, impacts, or exit-code mapping
- installer or release artifact verification
- Debian, Homebrew, or APT packaging

## Security Handling

Security reports are handled privately through GitHub Security Advisories.
Maintainers should avoid asking reporters to post sensitive logs or bundles in
public issues.

Confirmed vulnerabilities need:

- a fix
- a regression test where practical
- a changelog entry
- advisory publication when the fix is released

## Release Authority

Only maintainers should publish final release artifacts or package-channel
updates. Release candidates may be used for validation, but they should not be
promoted to Debian mentors, Homebrew, or APT channels as stable artifacts.
