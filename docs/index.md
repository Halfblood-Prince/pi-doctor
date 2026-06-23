# pi-doctor

Human-first Raspberry Pi diagnostics.

`pi-doctor` is a read-only CLI for checking common Raspberry Pi problems such as:

- identity mismatches
- thermal throttling
- `config.txt` drift
- GPIO conflicts
- camera detection problems
- Python environment pitfalls

## Quick Start

Run the main diagnostics:

```bash
pi-doctor check
```

Emit machine-readable output:

```bash
pi-doctor --json check
```

Bound external probe commands:

```bash
pi-doctor --timeout 5 check
```

Run focused help:

```bash
pi-doctor explain throttling
pi-doctor doctor gpio
pi-doctor support-bundle
```

## Documentation Map

- [Getting Started](getting-started.md): installation, core commands, and local docs usage
- [CLI Contract](cli-contract.md): stable automation-facing behavior
- [Exit Codes](exit-codes.md): process exit semantics
- [JSON Schema](json-schema.md): `check --json` structure and guarantees
- [Hardware Matrix](hardware-matrix.md): fixture strategy for parser coverage

## Automation Guidance

If you are integrating with `pi-doctor` from scripts or CI:

- prefer `pi-doctor check --json`
- gate behavior on `schema_version`
- inspect `probe_health` before treating missing data as healthy
- use exit codes only for coarse status
- ignore unknown JSON fields
