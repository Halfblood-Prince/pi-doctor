# Contributing

Thanks for helping make `pi-doctor` more reliable on real Raspberry Pi systems.

## Before Opening Work

- Check existing issues and known limitations.
- Do not include private email addresses, hostnames, serial numbers, tokens, or
  raw support bundles in public issues or commits.
- Keep fixtures sanitized and small enough for review.

## Local Checks

Run the same classes of checks as CI when the local toolchain is available:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
mkdocs build --strict
python3 scripts/validate-report-schema.py \
  --schema schema/pi-doctor-report.v1.schema.json \
  --registry schema/finding-registry.v1.json \
  path/to/report.json
```

## Change Guidelines

- Keep probe behavior read-only.
- Preserve deterministic JSON ordering.
- Add or update tests for parser changes, finding changes, and CLI contract
  changes.
- Update the JSON schema and finding registry when machine output changes.
- Bump the schema version for removals or semantic changes.
- Update documentation when commands, output, privacy behavior, or packaging
  behavior changes.

## Versioning

The package follows semantic versioning. Version `1.0.0` is the first stable
release, and the CLI and JSON contracts are deliberately frozen for the stable
`1.x` line. Machine-readable breaking changes require a schema-version bump and
migration notes.

## Pull Requests

PRs should include:

- a short summary of user-visible behavior
- the checks run locally
- any known hardware or OS validation gaps
- privacy review notes when support bundles, logs, or fixtures are touched
